#define WIN32_LEAN_AND_MEAN
#include <windows.h>
#include <tlhelp32.h>
#include <dbghelp.h>
#include <cstdio>
#include <cstdlib>
#include <cwchar>

struct Resume {
    HANDLE thread;
    ~Resume() { ResumeThread(thread); }
};
int main(int argc, char** argv) {
    if (argc != 2) return 2;
    DWORD pid = static_cast<DWORD>(std::strtoul(argv[1], nullptr, 10));
    HANDLE process = OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, FALSE, pid);
    if (!process) return 3;
    wchar_t path[32768]; DWORD length = 32768;
    if (!QueryFullProcessImageNameW(process, 0, path, &length)) return 4;
    const wchar_t* name = std::wcsrchr(path, L'\\');
    if (!name || std::wcscmp(name + 1, L"library_publication_audit.exe")) return 5;
    HANDLE threads = CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0);
    THREADENTRY32 entry{}; entry.dwSize = sizeof(entry);
    DWORD selected = 0; ULONGLONG highest = 0;
    if (Thread32First(threads, &entry)) do {
        if (entry.th32OwnerProcessID != pid) continue;
        HANDLE thread = OpenThread(THREAD_QUERY_INFORMATION, FALSE, entry.th32ThreadID);
        FILETIME created{}, ended{}, kernel{}, user{};
        if (thread && GetThreadTimes(thread, &created, &ended, &kernel, &user)) {
            ULONGLONG cpu = (static_cast<ULONGLONG>(user.dwHighDateTime) << 32) | user.dwLowDateTime;
            if (cpu >= highest) { highest = cpu; selected = entry.th32ThreadID; }
        }
        if (thread) CloseHandle(thread);
    } while (Thread32Next(threads, &entry));
    CloseHandle(threads);
    SymSetOptions(SYMOPT_DEFERRED_LOADS | SYMOPT_UNDNAME | SYMOPT_FAIL_CRITICAL_ERRORS | SYMOPT_NO_PROMPTS);
    if (!SymInitialize(process, "target\\release\\examples", TRUE)) return 6;
    HANDLE thread = OpenThread(THREAD_GET_CONTEXT | THREAD_SUSPEND_RESUME | THREAD_QUERY_INFORMATION, FALSE, selected);
    if (!thread) return 7;
    if (SuspendThread(thread) == static_cast<DWORD>(-1)) return 8;
    {
        Resume resume{thread};
        CONTEXT context{}; context.ContextFlags = CONTEXT_FULL;
        if (!GetThreadContext(thread, &context)) return 9;
        STACKFRAME64 frame{};
        frame.AddrPC.Offset = context.Rip; frame.AddrPC.Mode = AddrModeFlat;
        frame.AddrStack.Offset = context.Rsp; frame.AddrStack.Mode = AddrModeFlat;
        frame.AddrFrame.Offset = context.Rbp; frame.AddrFrame.Mode = AddrModeFlat;
        std::printf("pid=%lu thread=%lu user_100ns=%llu\n", pid, selected, highest);
        for (int i = 0; i < 40 && frame.AddrPC.Offset; ++i) {
            alignas(SYMBOL_INFO) unsigned char buffer[sizeof(SYMBOL_INFO) + MAX_SYM_NAME]{};
            auto symbol = reinterpret_cast<SYMBOL_INFO*>(buffer);
            symbol->SizeOfStruct = sizeof(SYMBOL_INFO); symbol->MaxNameLen = MAX_SYM_NAME;
            DWORD64 displacement = 0;
            if (SymFromAddr(process, frame.AddrPC.Offset, &displacement, symbol))
                std::printf("%02d %016llx %s + %llu\n", i, frame.AddrPC.Offset, symbol->Name, displacement);
            else std::printf("%02d %016llx (symbol unavailable)\n", i, frame.AddrPC.Offset);
            if (!StackWalk64(IMAGE_FILE_MACHINE_AMD64, process, thread, &frame, &context,
                             nullptr, SymFunctionTableAccess64, SymGetModuleBase64, nullptr)) break;
        }
    }
    CloseHandle(thread); SymCleanup(process); CloseHandle(process);
    return 0;
}

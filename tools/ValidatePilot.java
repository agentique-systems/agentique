// Independent verification harness for the published SysML v2 pilot, not Engine code.
import java.nio.file.*;
import java.util.*;
import org.omg.sysml.interactive.SysMLInteractive;
import org.eclipse.xtext.resource.XtextResource;
import org.eclipse.xtext.validation.*;
import org.eclipse.xtext.util.CancelIndicator;
import com.google.gson.GsonBuilder;
import org.omg.sysml.lang.sysml.Feature;

public class ValidatePilot {
    public static void main(String[] args) throws Exception {
        var sysml = SysMLInteractive.createInstance();
        sysml.loadLibrary(args[0]);
        var resources = new LinkedHashMap<String, XtextResource>();
        for (int i = 2; i < args.length; i++) {
            sysml.next(args[i].endsWith(".kerml") ? ".kerml" : ".sysml");
            sysml.parse(Files.readString(Path.of(args[i])));
            sysml.addResourceToIndex(sysml.getResource());
            resources.put(args[i], (XtextResource) sysml.getResource());
        }
        var issues = new ArrayList<Map<String,Object>>();
        for (var entry : resources.entrySet()) {
            var validator = entry.getValue().getResourceServiceProvider().get(IResourceValidator.class);
            for (var issue : validator.validate(entry.getValue(), CheckMode.ALL, CancelIndicator.NullImpl)) {
                var record = new LinkedHashMap<String,Object>();
                record.put("file",entry.getKey()); record.put("severity",issue.getSeverity().toString());
                record.put("code",issue.getCode()); record.put("message",issue.getMessage());
                record.put("line",issue.getLineNumber()); record.put("column",issue.getColumn());
                if(issue.getUriToProblem()!=null) {
                    var problem=entry.getValue().getEObject(issue.getUriToProblem().fragment());
                    record.put("metaclass",problem==null?null:problem.eClass().getName());
                    if(problem instanceof Feature feature) {
                        record.put("name",feature.getQualifiedName()); record.put("direction",String.valueOf(feature.getDirection()));
                        record.put("redefines",feature.getOwnedRedefinition().stream().map(r->r.getRedefinedFeature()).filter(Objects::nonNull).map(f->String.valueOf(f.getQualifiedName())+" direction="+f.getDirection()).toList());
                    }
                }
                issues.add(record);
            }
        }
        var report = new LinkedHashMap<String,Object>();
        report.put("validator", "OMG SysML v2 pilot 0.59.0 / CheckMode.ALL");
        report.put("files",resources.keySet()); report.put("issues",issues);
        long errors = issues.stream().filter(i -> i.get("severity").equals("ERROR")).count();
        report.put("errors",errors);
        Files.writeString(Path.of(args[1]),new GsonBuilder().setPrettyPrinting().create().toJson(report)+"\n");
        System.out.println("Pilot errors: " + errors);
        System.exit(errors == 0 ? 0 : 1);
    }
}

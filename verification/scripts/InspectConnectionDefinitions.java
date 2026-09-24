// Independent, read-only reference implementation inspection. Not a build step.
import java.nio.file.*;
import java.util.*;
import org.omg.sysml.interactive.SysMLInteractive;
import org.omg.sysml.lang.sysml.*;
import com.google.gson.GsonBuilder;
import org.omg.sysml.util.FeatureUtil;
import org.eclipse.xtext.resource.XtextResource;
import org.eclipse.xtext.validation.*;
import org.eclipse.xtext.util.CancelIndicator;

public class InspectConnectionDefinitions {
    static List<String> names(Collection<? extends Element> elements) {
        return elements.stream().map(e -> e.eClass().getName() + ":" + e.getQualifiedName()).toList();
    }
    public static void main(String[] args) throws Exception {
        var sysml = SysMLInteractive.createInstance();
        sysml.loadLibrary(args[0]);
        sysml.next(".sysml");
        sysml.parse(Files.readString(Path.of(args[1])));
        sysml.addResourceToIndex(sysml.getResource());
        System.out.println("Parse errors: " + sysml.getResource().getErrors());
        var output = new ArrayList<Map<String, Object>>();
        var contents = sysml.getResource().getAllContents();
        while (contents.hasNext()) {
            if (!(contents.next() instanceof ConnectionUsage connection)) continue;
            System.out.println("Connection " + connection.getQualifiedName() + " types " + names(connection.getDefinition()));
            if (connection.getOwnedTyping().stream().noneMatch(t ->
                    t.getType() != null && "HappensDuring".equals(t.getType().getDeclaredName()))) continue;
            var row = new LinkedHashMap<String, Object>();
            row.put("owner", connection.getOwner().getQualifiedName());
            row.put("directTypes", names(connection.getOwnedTyping().stream().map(t -> t.getType()).toList()));
            row.put("rawFeatureTypes", names(FeatureUtil.getAllTypesOf(connection)));
            row.put("definition", names(connection.getDefinition()));
            row.put("occurrenceDefinition", names(connection.getOccurrenceDefinition()));
            row.put("itemDefinition", names(connection.getItemDefinition()));
            row.put("partDefinition", names(connection.getPartDefinition()));
            row.put("connectionDefinition", names(connection.getConnectionDefinition()));
            row.put("ends", names(connection.getConnectorEnd()));
            output.add(row);
        }
        var resource = (XtextResource)sysml.getResource();
        var issues = resource.getResourceServiceProvider().get(IResourceValidator.class)
            .validate(resource, CheckMode.ALL, CancelIndicator.NullImpl);
        System.out.println("Validation issues: " + issues);
        Files.writeString(Path.of(args[2]), new GsonBuilder().setPrettyPrinting().create().toJson(output) + "\n");
        System.out.println("Inspected HappensDuring connections: " + output.size());
        if (output.size() != 2) System.exit(1);
    }
}

import { useState } from "react";
type Element = {
  id: string;
  name: string | null;
  kind: string;
  qualified_name: string;
  references: { role: string; target: string | null; path: string }[];
};
export function ModelEdits({
  selected,
  elements,
  disabled,
  onPropose,
}: {
  selected: Element;
  elements: Element[];
  disabled: boolean;
  onPropose: (edit: unknown) => void;
}) {
  const [target, setTarget] = useState(""),
    [value, setValue] = useState(""),
    [valueKind, setValueKind] = useState("string"),
    [connectionEnd, setConnectionEnd] = useState("");
  const definitions = elements.filter((e) => e.kind.endsWith("Definition")),
    owners = elements.filter(
      (e) => e.kind === "Package" || e.kind === "PartDefinition",
    ),
    ports = elements.filter((e) => e.kind === "PortUsage");
  return (
    <details>
      <summary>Type, value, ownership and connection edits</summary>
      <p className="editing-help">
        Each edit creates a candidate revision for validation and review.
      </p>
      {selected.kind === "AttributeUsage" && (
        <form
          onSubmit={(e) => {
            e.preventDefault();
            let parsed: unknown = value;
            if (valueKind === "boolean") {
              if (!["true", "false"].includes(value)) {
                return;
              }
              parsed = value === "true";
            }
            onPropose({
              kind: "set_value",
              element_id: selected.id,
              value: { kind: valueKind, value: parsed },
            });
          }}
        >
          <label htmlFor="attribute-kind">Scalar value type</label>
          <select
            id="attribute-kind"
            value={valueKind}
            onChange={(e) => setValueKind(e.target.value)}
          >
            <option value="string">String</option>
            <option value="integer">Integer (exact)</option>
            <option value="boolean">Boolean</option>
          </select>
          <label htmlFor="attribute-value">Attribute value</label>
          <input
            id="attribute-value"
            value={value}
            onChange={(e) => setValue(e.target.value)}
            required
          />
          <button disabled={disabled}>Review value</button>
        </form>
      )}
      <label htmlFor="edit-target">Destination / type</label>
      <select
        id="edit-target"
        value={target}
        onChange={(e) => setTarget(e.target.value)}
      >
        <option value="">Select an element</option>
        <optgroup label="Definitions">
          {definitions.map((e) => (
            <option key={e.id} value={e.id}>
              {e.qualified_name}
            </option>
          ))}
        </optgroup>
        <optgroup label="Packages">
          {owners
            .filter((e) => e.kind === "Package")
            .map((e) => (
              <option key={e.id} value={e.id}>
                {e.qualified_name}
              </option>
            ))}
        </optgroup>
      </select>
      <div className="edit-actions">
        <button
          disabled={
            disabled ||
            !target ||
            !selected.references.some((r) => r.role === "type")
          }
          onClick={() =>
            onPropose({
              kind: "set_type",
              element_id: selected.id,
              type_id: target,
            })
          }
        >
          Review type change
        </button>
        <button
          disabled={disabled || !target}
          onClick={() =>
            onPropose({
              kind: "move",
              element_id: selected.id,
              new_owner_id: target,
            })
          }
        >
          Review move
        </button>
      </div>
      {selected.kind === "PortUsage" && (
        <>
          <label htmlFor="connect-end">Connect selected port to</label>
          <select
            id="connect-end"
            value={connectionEnd}
            onChange={(e) => setConnectionEnd(e.target.value)}
          >
            <option value="">Select a port</option>
            {ports
              .filter((e) => e.id !== selected.id)
              .map((e) => (
                <option key={e.id} value={e.id}>
                  {e.qualified_name}
                </option>
              ))}
          </select>
          <button
            disabled={disabled || !target || !connectionEnd}
            onClick={() =>
              onPropose({
                kind: "connect",
                owner_id: target,
                source_id: selected.id,
                target_id: connectionEnd,
              })
            }
          >
            Review connection in destination
          </button>
        </>
      )}
    </details>
  );
}

import { createEventListener } from "@solid-primitives/event-listener";

export function isInputFocused(): boolean {
	const el = document.activeElement;
	if (!el) return false;
	const tagName = el.tagName.toLowerCase();
	const isContentEditable = el.getAttribute("contenteditable") === "true";
	return tagName === "input" || tagName === "textarea" || isContentEditable;
}

export type ShortcutBinding = {
	combo: string; // e.g. "Mod+=", "Mod+-", "Space", "S", "C"
	handler: (e: KeyboardEvent) => void | Promise<void>;
	preventDefault?: boolean; // default: true
	when?: () => boolean; // optional enablement gate
};

const isMod = (e: KeyboardEvent) => e.metaKey || e.ctrlKey; // treat Cmd/Ctrl as Mod

function normalizeCombo(e: KeyboardEvent): string {
	const parts: string[] = [];

	if (e.ctrlKey && !e.metaKey) parts.push("Ctrl");
	if (e.metaKey) parts.push("Mod");
	if (e.shiftKey) parts.push("Shift");
	if (e.altKey) parts.push("Alt");

	let key: string;

	if (e.key.length === 1 && !e.ctrlKey && !e.metaKey && e.code !== "Space") {
		key = e.key.toUpperCase();
	} else {
		switch (e.code) {
			case "Equal":
				key = "=";
				break;
			case "Minus":
				key = "-";
				break;
			case "Space":
				key = "Space";
				break;
			case "Escape":
				key = "Escape";
				break;
			case "Backspace":
				key = "Backspace";
				break;
			case "Delete":
				key = "Delete";
				break;
			default:
				key = e.code.startsWith("Key") ? e.code.slice(3) : e.code;
		}
	}

	parts.push(key);
	return parts.join("+");
}

export function useEditorShortcuts(
	getScopeActive: () => boolean,
	bindings: ShortcutBinding[],
) {
	const map = new Map<string, ShortcutBinding>(
		bindings.map((b) => [b.combo, b]),
	);

	createEventListener(document, "keydown", async (e: KeyboardEvent) => {
		// Basic guards
		if (!getScopeActive()) return;
		if (e.repeat) return;

		const binding = map.get(normalizeCombo(e));
		if (!binding) return;
		if (binding.when && !binding.when()) return;

		if (binding.preventDefault !== false) e.preventDefault();

		await binding.handler(e);
	});
}

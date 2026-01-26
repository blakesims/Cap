import { For, Show } from "solid-js";

const shortcutGroups = [
	{
		title: "Playback",
		shortcuts: [
			{ key: "Space", description: "Play / Pause" },
			{ key: "K", description: "Pause" },
			{ key: "Ctrl+L", description: "Increase speed" },
			{ key: "Ctrl+J", description: "Decrease speed" },
		],
	},
	{
		title: "Navigation",
		shortcuts: [
			{ key: "H", description: "Step back 1 frame" },
			{ key: "L", description: "Step forward 1 frame" },
			{ key: "Shift+H", description: "Step back 1 second" },
			{ key: "Shift+L", description: "Step forward 1 second" },
			{ key: "W", description: "Jump to next segment" },
			{ key: "B", description: "Jump to previous segment" },
			{ key: "0", description: "Jump to start" },
			{ key: "$", description: "Jump to end" },
		],
	},
	{
		title: "Editing",
		shortcuts: [
			{ key: "C", description: "Split at playhead" },
			{ key: "Delete / X", description: "Delete segment or IN/OUT region" },
			{ key: "I", description: "Set IN point" },
			{ key: "O", description: "Set OUT point" },
			{ key: "M", description: "Set mark" },
			{ key: "' or `", description: "Jump to mark" },
			{ key: "Escape", description: "Clear selection and markers" },
		],
	},
	{
		title: "View",
		shortcuts: [
			{ key: "Cmd/Ctrl + =", description: "Zoom in timeline" },
			{ key: "Cmd/Ctrl + -", description: "Zoom out timeline" },
			{ key: "?", description: "Show this help" },
		],
	},
];

export function KeyboardShortcutsModal(props: {
	open: boolean;
	onClose: () => void;
}) {
	return (
		<Show when={props.open}>
			<div
				class="fixed inset-0 bg-black/60 flex items-center justify-center z-50"
				onClick={(e) => {
					if (e.target === e.currentTarget) props.onClose();
				}}
				onKeyDown={(e) => {
					if (e.key === "Escape") props.onClose();
				}}
			>
				<div class="bg-gray-50 rounded-xl p-6 max-w-2xl w-full mx-4 max-h-[80vh] overflow-auto shadow-xl">
					<div class="flex justify-between items-center mb-6">
						<h2 class="text-xl font-semibold text-gray-500">
							Keyboard Shortcuts
						</h2>
						<button
							onClick={props.onClose}
							class="text-gray-400 hover:text-gray-500 transition-colors"
						>
							<svg
								class="w-6 h-6"
								fill="none"
								viewBox="0 0 24 24"
								stroke="currentColor"
							>
								<path
									stroke-linecap="round"
									stroke-linejoin="round"
									stroke-width="2"
									d="M6 18L18 6M6 6l12 12"
								/>
							</svg>
						</button>
					</div>

					<div class="grid grid-cols-1 md:grid-cols-2 gap-6">
						<For each={shortcutGroups}>
							{(group) => (
								<div>
									<h3 class="text-sm font-medium text-gray-400 uppercase tracking-wider mb-3">
										{group.title}
									</h3>
									<div class="space-y-2">
										<For each={group.shortcuts}>
											{(shortcut) => (
												<div class="flex justify-between items-center">
													<span class="text-gray-500">
														{shortcut.description}
													</span>
													<kbd class="px-2 py-1 bg-gray-100 rounded text-sm font-mono text-gray-500 border border-gray-200">
														{shortcut.key}
													</kbd>
												</div>
											)}
										</For>
									</div>
								</div>
							)}
						</For>
					</div>
				</div>
			</div>
		</Show>
	);
}

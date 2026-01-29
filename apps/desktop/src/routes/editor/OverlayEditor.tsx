import { Button } from "@cap/ui-solid";
import { Select as KSelect } from "@kobalte/core/select";
import { cx } from "cva";
import { createEffect, createMemo, For, on, Show } from "solid-js";
import { createStore, produce, reconcile } from "solid-js/store";

import { useEditorContext } from "./context";
import type {
	OverlayItem,
	OverlayItemStyle,
	OverlaySegment,
	OverlayType,
} from "./Timeline/OverlayTrack";
import { Dialog, dropdownContainerClasses, Input, topSlideAnimateClasses } from "./ui";

const OVERLAY_TYPE_OPTIONS: { value: OverlayType; label: string }[] = [
	{ value: "split", label: "Split (50/50)" },
	{ value: "fullScreen", label: "Full Screen" },
];

const ITEM_STYLE_OPTIONS: { value: OverlayItemStyle; label: string }[] = [
	{ value: "title", label: "Title" },
	{ value: "bullet", label: "Bullet" },
	{ value: "numbered", label: "Numbered" },
];

type OverlayEditorProps = {
	open: boolean;
	segmentIndex: number;
	onClose: () => void;
};

export function OverlayEditor(props: OverlayEditorProps) {
	const { project, setProject, projectHistory } = useEditorContext();

	const segment = createMemo((): OverlaySegment | undefined => {
		const overlays = (project.timeline?.overlaySegments as
			| OverlaySegment[]
			| undefined) ?? [];
		return overlays[props.segmentIndex];
	});

	const [localOverlay, setLocalOverlay] = createStore<{
		overlayType: OverlayType;
		items: OverlayItem[];
	}>({
		overlayType: "split",
		items: [],
	});

	createEffect(
		on(
			() => ({ open: props.open, seg: segment() }),
			({ open, seg }) => {
				if (open && seg) {
					setLocalOverlay(
						reconcile({
							overlayType: seg.overlayType,
							items: seg.items.map((item) => ({ ...item })),
						}),
					);
				}
			},
		),
	);

	const handleSave = () => {
		if (!segment()) return;

		setProject(
			"timeline",
			"overlaySegments" as keyof typeof project.timeline,
			props.segmentIndex as never,
			produce((seg: OverlaySegment) => {
				seg.overlayType = localOverlay.overlayType;
				seg.items = localOverlay.items.map((item) => ({ ...item }));
			}) as never,
		);

		props.onClose();
	};

	const handleAddItem = () => {
		const maxDelay = localOverlay.items.reduce(
			(max, item) => Math.max(max, item.delay),
			0,
		);
		setLocalOverlay(
			"items",
			produce((items) => {
				items.push({
					delay: maxDelay + 0.5,
					content: "New item",
					style: "bullet",
				});
			}),
		);
	};

	const handleRemoveItem = (index: number) => {
		if (localOverlay.items.length <= 1) return;
		setLocalOverlay(
			"items",
			produce((items) => {
				items.splice(index, 1);
			}),
		);
	};

	const handleMoveItem = (index: number, direction: -1 | 1) => {
		const newIndex = index + direction;
		if (newIndex < 0 || newIndex >= localOverlay.items.length) return;
		setLocalOverlay(
			"items",
			produce((items) => {
				const temp = items[index];
				items[index] = items[newIndex];
				items[newIndex] = temp;
			}),
		);
	};

	const handleItemChange = (
		index: number,
		field: keyof OverlayItem,
		value: string | number,
	) => {
		setLocalOverlay("items", index, field as never, value as never);
	};

	const segmentDuration = createMemo(() => {
		const seg = segment();
		if (!seg) return 0;
		return seg.end - seg.start;
	});

	return (
		<Dialog.Root open={props.open} onOpenChange={(open) => !open && props.onClose()} size="lg">
			<Dialog.Header>
				<span class="text-gray-12 font-medium">Edit Overlay</span>
			</Dialog.Header>

			<Dialog.Content class="max-h-[60vh] overflow-y-auto">
				<div class="flex flex-col gap-6">
					<div class="flex flex-col gap-2">
						<label class="text-sm font-medium text-gray-12">Overlay Type</label>
						<KSelect
							value={localOverlay.overlayType}
							onChange={(value) => {
								if (value) setLocalOverlay("overlayType", value);
							}}
							options={OVERLAY_TYPE_OPTIONS.map((o) => o.value)}
							itemComponent={(itemProps) => (
								<KSelect.Item
									item={itemProps.item}
									class="flex items-center px-3 py-2 text-sm cursor-pointer rounded-lg outline-none ui-highlighted:bg-gray-3 text-gray-12"
								>
									<KSelect.ItemLabel>
										{OVERLAY_TYPE_OPTIONS.find((o) => o.value === itemProps.item.rawValue)?.label}
									</KSelect.ItemLabel>
								</KSelect.Item>
							)}
						>
							<KSelect.Trigger class="flex items-center justify-between w-full px-3 py-2 text-sm border border-gray-3 rounded-lg bg-gray-2 hover:bg-gray-3 transition-colors text-gray-12">
								<KSelect.Value<OverlayType>>
									{(state) =>
										OVERLAY_TYPE_OPTIONS.find((o) => o.value === state.selectedOption())?.label
									}
								</KSelect.Value>
								<KSelect.Icon class="text-gray-10">
									<IconLucideChevronDown class="size-4" />
								</KSelect.Icon>
							</KSelect.Trigger>
							<KSelect.Portal>
								<KSelect.Content class={cx(dropdownContainerClasses, topSlideAnimateClasses, "w-[200px]")}>
									<KSelect.Listbox class="p-1" />
								</KSelect.Content>
							</KSelect.Portal>
						</KSelect>
						<p class="text-xs text-gray-10">
							{localOverlay.overlayType === "split"
								? "Camera on right 50%, background + text on left 50%"
								: "PiP camera in corner, full-width text overlay"}
						</p>
					</div>

					<div class="flex flex-col gap-3">
						<div class="flex items-center justify-between">
							<label class="text-sm font-medium text-gray-12">Items</label>
							<Button variant="secondary" size="xs" onClick={handleAddItem}>
								<IconLucidePlus class="size-3.5 mr-1" />
								Add Item
							</Button>
						</div>

						<Show
							when={localOverlay.items.length > 0}
							fallback={
								<div class="text-center py-8 text-gray-10 text-sm border border-dashed border-gray-4 rounded-lg">
									No items yet. Click "Add Item" to get started.
								</div>
							}
						>
							<div class="flex flex-col gap-3">
								<For each={localOverlay.items}>
									{(item, index) => (
										<div class="flex flex-col gap-2 p-3 border border-gray-3 rounded-lg bg-gray-2/50">
											<div class="flex items-center gap-2">
												<div class="flex flex-col gap-0.5">
													<button
														type="button"
														class="p-0.5 rounded hover:bg-gray-3 text-gray-10 hover:text-gray-12 disabled:opacity-30 disabled:cursor-not-allowed transition-colors"
														onClick={() => handleMoveItem(index(), -1)}
														disabled={index() === 0}
														title="Move up"
													>
														<IconLucideChevronUp class="size-3.5" />
													</button>
													<button
														type="button"
														class="p-0.5 rounded hover:bg-gray-3 text-gray-10 hover:text-gray-12 disabled:opacity-30 disabled:cursor-not-allowed transition-colors"
														onClick={() => handleMoveItem(index(), 1)}
														disabled={index() === localOverlay.items.length - 1}
														title="Move down"
													>
														<IconLucideChevronDown class="size-3.5" />
													</button>
												</div>

												<span class="text-xs text-gray-10 font-medium w-5">
													{index() + 1}.
												</span>

												<div class="flex-1">
													<Input
														value={item.content}
														onInput={(e) =>
															handleItemChange(index(), "content", e.currentTarget.value)
														}
														placeholder="Item text"
														class="w-full"
													/>
												</div>

												<button
													type="button"
													class="p-1.5 rounded hover:bg-red-3 text-gray-10 hover:text-red-9 disabled:opacity-30 disabled:cursor-not-allowed transition-colors"
													onClick={() => handleRemoveItem(index())}
													disabled={localOverlay.items.length <= 1}
													title="Remove item"
												>
													<IconLucideTrash2 class="size-4" />
												</button>
											</div>

											<div class="flex items-center gap-3 ml-7">
												<div class="flex items-center gap-2">
													<label class="text-xs text-gray-10 whitespace-nowrap">
														Delay (s):
													</label>
													<input
														type="number"
														step="0.1"
														min="0"
														max={segmentDuration()}
														value={item.delay.toFixed(1)}
														onInput={(e) => {
															const value = Number.parseFloat(e.currentTarget.value);
															if (!Number.isNaN(value) && value >= 0) {
																handleItemChange(index(), "delay", value);
															}
														}}
														class="w-16 px-2 py-1 text-xs border border-gray-3 rounded bg-gray-2 text-gray-12 focus:outline-none focus:ring-1 focus:ring-blue-9"
													/>
												</div>

												<div class="flex items-center gap-2">
													<label class="text-xs text-gray-10">Style:</label>
													<KSelect
														value={item.style}
														onChange={(value) => {
															if (value) handleItemChange(index(), "style", value);
														}}
														options={ITEM_STYLE_OPTIONS.map((o) => o.value)}
														itemComponent={(itemProps) => (
															<KSelect.Item
																item={itemProps.item}
																class="flex items-center px-2 py-1.5 text-xs cursor-pointer rounded outline-none ui-highlighted:bg-gray-3 text-gray-12"
															>
																<KSelect.ItemLabel>
																	{ITEM_STYLE_OPTIONS.find(
																		(o) => o.value === itemProps.item.rawValue,
																	)?.label}
																</KSelect.ItemLabel>
															</KSelect.Item>
														)}
													>
														<KSelect.Trigger class="flex items-center justify-between gap-1 px-2 py-1 text-xs border border-gray-3 rounded bg-gray-2 hover:bg-gray-3 transition-colors text-gray-12 min-w-[90px]">
															<KSelect.Value<OverlayItemStyle>>
																{(state) =>
																	ITEM_STYLE_OPTIONS.find(
																		(o) => o.value === state.selectedOption(),
																	)?.label
																}
															</KSelect.Value>
															<KSelect.Icon class="text-gray-10">
																<IconLucideChevronDown class="size-3" />
															</KSelect.Icon>
														</KSelect.Trigger>
														<KSelect.Portal>
															<KSelect.Content
																class={cx(
																	dropdownContainerClasses,
																	topSlideAnimateClasses,
																	"w-[100px]",
																)}
															>
																<KSelect.Listbox class="p-1" />
															</KSelect.Content>
														</KSelect.Portal>
													</KSelect>
												</div>
											</div>

											<Show when={item.delay >= segmentDuration()}>
												<div class="ml-7 text-xs text-amber-9 flex items-center gap-1">
													<IconLucideAlertTriangle class="size-3" />
													Delay exceeds segment duration ({segmentDuration().toFixed(1)}s)
												</div>
											</Show>
										</div>
									)}
								</For>
							</div>
						</Show>
					</div>
				</div>
			</Dialog.Content>

			<Dialog.Footer>
				<Button variant="gray" onClick={props.onClose}>
					Cancel
				</Button>
				<Button variant="primary" onClick={handleSave}>
					Save Changes
				</Button>
			</Dialog.Footer>
		</Dialog.Root>
	);
}

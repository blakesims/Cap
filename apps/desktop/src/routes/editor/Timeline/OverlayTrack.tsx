import { createEventListenerMap } from "@solid-primitives/event-listener";
import { cx } from "cva";
import { createMemo, createRoot, For, Show } from "solid-js";
import { produce } from "solid-js/store";

import { useEditorContext } from "../context";
import {
	useSegmentContext,
	useTimelineContext,
	useTrackContext,
} from "./context";
import { SegmentContent, SegmentHandle, SegmentRoot, TrackRoot } from "./Track";

export type OverlayType = "split" | "fullScreen";

export type OverlayItemStyle = "title" | "bullet" | "numbered";

export type OverlayItem = {
	delay: number;
	content: string;
	style: OverlayItemStyle;
};

export type OverlaySegment = {
	start: number;
	end: number;
	overlayType: OverlayType;
	items: OverlayItem[];
};

export type OverlaySegmentDragState =
	| { type: "idle" }
	| { type: "movePending" }
	| { type: "moving" };

const MIN_SEGMENT_SECS = 1;
const MIN_SEGMENT_PIXELS = 80;

export function OverlayTrack(props: {
	onDragStateChanged: (v: OverlaySegmentDragState) => void;
	handleUpdatePlayhead: (e: MouseEvent) => void;
	onDoubleClick?: (index: number) => void;
}) {
	const {
		project,
		setProject,
		editorState,
		setEditorState,
		totalDuration,
		projectHistory,
	} = useEditorContext();
	const { secsPerPixel, timelineBounds } = useTimelineContext();

	const minDuration = () =>
		Math.max(MIN_SEGMENT_SECS, secsPerPixel() * MIN_SEGMENT_PIXELS);

	const overlaySegments = (): OverlaySegment[] =>
		(project.timeline?.overlaySegments as OverlaySegment[] | undefined) ?? [];

	const neighborBounds = (index: number) => {
		const segments = overlaySegments();
		return {
			prevEnd: segments[index - 1]?.end ?? 0,
			nextStart: segments[index + 1]?.start ?? totalDuration(),
		};
	};

	const findPlacement = (time: number, length: number) => {
		const gaps: Array<{ start: number; end: number }> = [];
		const sorted = overlaySegments()
			.slice()
			.sort((a, b) => a.start - b.start);

		let cursor = 0;
		for (const segment of sorted) {
			if (segment.start - cursor >= length) {
				gaps.push({ start: cursor, end: segment.start });
			}
			cursor = Math.max(cursor, segment.end);
		}

		if (totalDuration() - cursor >= length) {
			gaps.push({ start: cursor, end: totalDuration() });
		}

		if (gaps.length === 0) return null;

		const maxStart = Math.max(totalDuration() - length, 0);
		const desiredStart = Math.min(Math.max(time - length / 2, 0), maxStart);

		const containingGap =
			gaps.find(
				(gap) => desiredStart >= gap.start && desiredStart + length <= gap.end,
			) ??
			gaps.find((gap) => gap.start >= desiredStart) ??
			gaps[gaps.length - 1];

		const start = Math.min(
			Math.max(desiredStart, containingGap.start),
			containingGap.end - length,
		);

		return { start, end: start + length };
	};

	const addSegmentAt = (time: number) => {
		const length = Math.min(minDuration(), totalDuration());
		if (length <= 0) return;

		const placement = findPlacement(time, length);
		if (!placement) return;

		const defaultOverlay: OverlaySegment = {
			start: placement.start,
			end: placement.end,
			overlayType: "split",
			items: [{ delay: 0, content: "New Overlay", style: "title" }],
		};

		setProject(
			"timeline",
			"overlaySegments" as keyof typeof project.timeline,
			produce((segments: OverlaySegment[] | undefined) => {
				const arr = segments ?? [];
				arr.push(defaultOverlay);
				arr.sort((a, b) => a.start - b.start);
				return arr;
			}) as never,
		);
	};

	const handleBackgroundMouseDown = (e: MouseEvent) => {
		if (e.button !== 0) return;
		if ((e.target as HTMLElement).closest("[data-overlay-segment]")) return;
		const timelineTime =
			editorState.previewTime ??
			editorState.playbackTime ??
			secsPerPixel() * (e.clientX - (timelineBounds.left ?? 0));
		addSegmentAt(timelineTime);
	};

	function createMouseDownDrag<T>(
		segmentIndex: () => number,
		setup: () => T,
		update: (e: MouseEvent, value: T, initialMouseX: number) => void,
	) {
		return (downEvent: MouseEvent) => {
			if (editorState.timeline.interactMode !== "seek") return;
			downEvent.stopPropagation();
			const initial = setup();
			let moved = false;
			let initialMouseX: number | null = null;

			const resumeHistory = projectHistory.pause();
			props.onDragStateChanged({ type: "movePending" });

			function finish(e: MouseEvent) {
				resumeHistory();
				if (!moved) {
					e.stopPropagation();
					const currentSelection = editorState.timeline.selection;
					const index = segmentIndex();
					const isMultiSelect = e.ctrlKey || e.metaKey;
					const isRangeSelect = e.shiftKey;

					if (isRangeSelect && currentSelection?.type === "overlay") {
						const existingIndices = currentSelection.indices;
						const lastIndex = existingIndices[existingIndices.length - 1];
						const start = Math.min(lastIndex, index);
						const end = Math.max(lastIndex, index);
						const rangeIndices: number[] = [];
						for (let idx = start; idx <= end; idx++) rangeIndices.push(idx);
						setEditorState("timeline", "selection", {
							type: "overlay",
							indices: rangeIndices,
						});
					} else if (isMultiSelect) {
						if (currentSelection?.type === "overlay") {
							const base = currentSelection.indices;
							const exists = base.includes(index);
							const next = exists
								? base.filter((i) => i !== index)
								: [...base, index];
							setEditorState(
								"timeline",
								"selection",
								next.length > 0
									? {
											type: "overlay",
											indices: next,
										}
									: null,
							);
						} else {
							setEditorState("timeline", "selection", {
								type: "overlay",
								indices: [index],
							});
						}
					} else {
						setEditorState("timeline", "selection", {
							type: "overlay",
							indices: [index],
						});
					}
					props.handleUpdatePlayhead(e);
				}
				props.onDragStateChanged({ type: "idle" });
			}

			function handleUpdate(event: MouseEvent) {
				if (Math.abs(event.clientX - downEvent.clientX) > 2) {
					if (!moved) {
						moved = true;
						initialMouseX = event.clientX;
						props.onDragStateChanged({ type: "moving" });
					}
				}

				if (initialMouseX === null) return;
				update(event, initial, initialMouseX);
			}

			createRoot((dispose) => {
				createEventListenerMap(window, {
					mousemove: (e) => handleUpdate(e),
					mouseup: (e) => {
						handleUpdate(e);
						finish(e);
						dispose();
					},
				});
			});
		};
	}

	const getOverlayIcon = (overlayType: OverlayType) => {
		switch (overlayType) {
			case "split":
				return <IconLucideColumns class="size-3.5" />;
			case "fullScreen":
				return <IconLucideMaximize class="size-3.5" />;
			default:
				return <IconLucideLayout class="size-3.5" />;
		}
	};

	const getOverlayLabel = (overlayType: OverlayType) => {
		switch (overlayType) {
			case "split":
				return "Split";
			case "fullScreen":
				return "Full";
			default:
				return "Overlay";
		}
	};

	const getGradientClasses = (overlayType: OverlayType) => {
		switch (overlayType) {
			case "split":
				return "bg-gradient-to-r from-[#C4501B] via-[#FA8C5C] to-[#C4501B] shadow-[inset_0_8px_12px_3px_rgba(255,255,255,0.2)]";
			case "fullScreen":
				return "bg-gradient-to-r from-[#1B8C7A] via-[#5CFAD4] to-[#1B8C7A] shadow-[inset_0_8px_12px_3px_rgba(255,255,255,0.2)]";
			default:
				return "bg-gradient-to-r from-[#C4501B] via-[#FA8C5C] to-[#C4501B] shadow-[inset_0_8px_12px_3px_rgba(255,255,255,0.2)]";
		}
	};

	return (
		<TrackRoot
			onMouseEnter={() =>
				setEditorState("timeline", "hoveredTrack", "overlay" as never)
			}
			onMouseLeave={() => setEditorState("timeline", "hoveredTrack", null)}
			onMouseDown={handleBackgroundMouseDown}
		>
			<For
				each={overlaySegments()}
				fallback={
					<div class="text-center text-sm text-[--text-tertiary] flex flex-col justify-center items-center inset-0 w-full bg-gray-3/20 dark:bg-gray-3/10 hover:bg-gray-3/30 dark:hover:bg-gray-3/20 transition-colors rounded-xl pointer-events-none">
						<div>Click to add overlay</div>
						<div class="text-[10px] text-[--text-tertiary]/40 mt-0.5">
							(Split screen with text or full-screen text)
						</div>
					</div>
				}
			>
				{(segment, i) => {
					const isSelected = createMemo(() => {
						const selection = editorState.timeline.selection;
						if (!selection || selection.type !== "overlay") return false;
						return (
							selection as { type: "overlay"; indices: number[] }
						).indices.includes(i());
					});

					const segmentWidth = () => segment.end - segment.start;

					const handleDoubleClick = (e: MouseEvent) => {
						e.stopPropagation();
						props.onDoubleClick?.(i());
					};

					return (
						<SegmentRoot
							data-overlay-segment
							data-index={i()}
							class={cx(
								"border duration-200 hover:border-white transition-colors group",
								getGradientClasses(segment.overlayType),
								isSelected() ? "border-white" : "border-transparent",
							)}
							innerClass="ring-orange-6"
							segment={segment}
							onMouseDown={(e) => {
								e.stopPropagation();
								if (editorState.timeline.interactMode === "split") {
									const rect = e.currentTarget.getBoundingClientRect();
									const fraction = (e.clientX - rect.left) / rect.width;
									const splitTime = fraction * segmentWidth();
									splitOverlaySegment(i(), splitTime);
								}
							}}
							onDblClick={handleDoubleClick}
						>
							<SegmentHandle
								position="start"
								onMouseDown={createMouseDownDrag(
									i,
									() => {
										const bounds = neighborBounds(i());
										const start = segment.start;
										const minValue = bounds.prevEnd;
										const maxValue = Math.max(
											minValue,
											Math.min(
												segment.end - minDuration(),
												bounds.nextStart - minDuration(),
											),
										);
										return { start, minValue, maxValue };
									},
									(e, value, initialMouseX) => {
										const delta = (e.clientX - initialMouseX) * secsPerPixel();
										const next = Math.max(
											value.minValue,
											Math.min(value.maxValue, value.start + delta),
										);
										setProject(
											"timeline",
											"overlaySegments" as keyof typeof project.timeline,
											i() as never,
											"start" as never,
											next as never,
										);
										setProject(
											"timeline",
											"overlaySegments" as keyof typeof project.timeline,
											produce((items: OverlaySegment[]) => {
												items.sort((a, b) => a.start - b.start);
											}) as never,
										);
									},
								)}
							/>
							<SegmentContent
								class="flex justify-center items-center cursor-grab px-3 overflow-hidden"
								onMouseDown={createMouseDownDrag(
									i,
									() => {
										const original = { ...segment };
										const bounds = neighborBounds(i());
										const minDelta = bounds.prevEnd - original.start;
										const maxDelta = bounds.nextStart - original.end;
										return {
											original,
											minDelta,
											maxDelta,
										};
									},
									(e, value, initialMouseX) => {
										const delta = (e.clientX - initialMouseX) * secsPerPixel();
										const lowerBound = Math.min(value.minDelta, value.maxDelta);
										const upperBound = Math.max(value.minDelta, value.maxDelta);
										const clampedDelta = Math.min(
											upperBound,
											Math.max(lowerBound, delta),
										);
										setProject(
											"timeline",
											"overlaySegments" as keyof typeof project.timeline,
											i() as never,
											{
												...value.original,
												start: value.original.start + clampedDelta,
												end: value.original.end + clampedDelta,
											} as never,
										);
										setProject(
											"timeline",
											"overlaySegments" as keyof typeof project.timeline,
											produce((items: OverlaySegment[]) => {
												items.sort((a, b) => a.start - b.start);
											}) as never,
										);
									},
								)}
							>
								{(() => {
									const ctx = useSegmentContext();

									return (
										<Show when={ctx.width() > 60}>
											<div class="flex flex-col gap-0.5 justify-center items-center text-xs text-gray-1 dark:text-gray-12 w-full min-w-0 overflow-hidden animate-in fade-in">
												<span class="opacity-70">Overlay</span>
												<div class="flex gap-1 items-center text-md w-full min-w-0 justify-center">
													{getOverlayIcon(segment.overlayType)}
													<Show when={ctx.width() > 100}>
														<span class="truncate">
															{getOverlayLabel(segment.overlayType)}
														</span>
													</Show>
												</div>
											</div>
										</Show>
									);
								})()}
							</SegmentContent>
							<SegmentHandle
								position="end"
								onMouseDown={createMouseDownDrag(
									i,
									() => {
										const bounds = neighborBounds(i());
										const end = segment.end;
										const minValue = segment.start + minDuration();
										const maxValue = Math.max(minValue, bounds.nextStart);
										return { end, minValue, maxValue };
									},
									(e, value, initialMouseX) => {
										const delta = (e.clientX - initialMouseX) * secsPerPixel();
										const next = Math.max(
											value.minValue,
											Math.min(value.maxValue, value.end + delta),
										);
										setProject(
											"timeline",
											"overlaySegments" as keyof typeof project.timeline,
											i() as never,
											"end" as never,
											next as never,
										);
										setProject(
											"timeline",
											"overlaySegments" as keyof typeof project.timeline,
											produce((items: OverlaySegment[]) => {
												items.sort((a, b) => a.start - b.start);
											}) as never,
										);
									},
								)}
							/>
						</SegmentRoot>
					);
				}}
			</For>
		</TrackRoot>
	);

	function splitOverlaySegment(index: number, time: number) {
		setProject(
			"timeline",
			"overlaySegments" as keyof typeof project.timeline,
			produce((segments: OverlaySegment[] | undefined) => {
				const segment = segments?.[index];
				if (!segment) return;

				const duration = segment.end - segment.start;
				const remaining = duration - time;
				if (time < 1 || remaining < 1) return;

				segments.splice(index + 1, 0, {
					...segment,
					start: segment.start + time,
					end: segment.end,
					items: segment.items.map((item) => ({
						...item,
						delay: Math.max(0, item.delay - time),
					})),
				});
				segments[index].end = segment.start + time;
			}) as never,
		);
	}
}

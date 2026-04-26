<script lang="ts">
	import { browser } from '$app/environment';
	import { Settings2, RotateCcw, X } from 'lucide-svelte';
	import { onMount } from 'svelte';
	import Panel from '$lib/components/Panel.svelte';
	import StatusPill from '$lib/components/StatusPill.svelte';
	import {
		chapterParagraphs,
		countWords,
		prettyEventLabel,
		summarizeEventPayload
	} from '$lib/workspace/presenters';
	import type { PageData } from './$types';

	let { data }: { data: PageData } = $props();

	const storageKey = $derived(`novelgraph:reading:${data.workspace.project.id}`);
	const settingsKey = $derived(`novelgraph:reading-settings:${data.workspace.project.id}`);
	let selectedChapterId = $state('');
	let searchQuery = $state('');
	let isReadingSettingsOpen = $state(false);
	let fontSizePx = $state(15);
	let lineHeightValue = $state(1.65);

	onMount(() => {
		if (!browser || data.workspace.chapters.length === 0) {
			return;
		}

		const savedChapterId = localStorage.getItem(storageKey);
		if (
			savedChapterId &&
			data.workspace.chapters.some((chapter) => chapter.id === savedChapterId)
		) {
			selectedChapterId = savedChapterId;
		}

		const savedSettings = localStorage.getItem(settingsKey);
		if (savedSettings) {
			try {
				const parsed = JSON.parse(savedSettings) as {
					fontSizePx?: number;
					lineHeightValue?: number;
				};
				if (typeof parsed.fontSizePx === 'number') {
					fontSizePx = parsed.fontSizePx;
				}
				if (typeof parsed.lineHeightValue === 'number') {
					lineHeightValue = parsed.lineHeightValue;
				}
			} catch {
				localStorage.removeItem(settingsKey);
			}
		}
	});

	$effect(() => {
		if (!selectedChapterId && data.workspace.chapters[0]) {
			selectedChapterId = data.workspace.chapters[0].id;
		}
	});

	$effect(() => {
		if (browser && selectedChapterId) {
			localStorage.setItem(storageKey, selectedChapterId);
		}
	});

	$effect(() => {
		if (browser) {
			localStorage.setItem(
				settingsKey,
				JSON.stringify({
					fontSizePx,
					lineHeightValue
				})
			);
		}
	});

	const activeChapter = $derived(
		data.workspace.chapters.find((chapter) => chapter.id === selectedChapterId) ??
			data.workspace.chapters[0] ??
			null
	);
	const activeParagraphs = $derived(activeChapter ? chapterParagraphs(activeChapter) : []);
	const hitCount = $derived(
		searchQuery.trim().length === 0
			? 0
			: activeParagraphs.filter((paragraph) =>
					paragraph.toLowerCase().includes(searchQuery.trim().toLowerCase())
				).length
	);

	function resetReadingSettings() {
		fontSizePx = 15;
		lineHeightValue = 1.65;
	}
</script>

<div class="page-stack">
	{#if data.workspace.chapters.length === 0}
		<div class="empty-note">
			Chưa có chương nào để đọc. Hãy import truyện trước khi dùng reading workspace.
		</div>
	{:else}
		<div class="page-grid page-grid--wide">
			<Panel subtitle="Persisted locally per project" title="Chapter list">
				<div class="chapter-stack">
					{#each data.workspace.chapters as chapter (chapter.id)}
						<button
							class:is-active={selectedChapterId === chapter.id}
							class="chapter-item"
							onclick={() => {
								selectedChapterId = chapter.id;
							}}
							type="button"
						>
							<div class="status-row">
								<div class="nav-link__title">{chapter.title}</div>
								<StatusPill label="Imported" tone="good" />
							</div>
							<div class="nav-link__meta">
								Chapter {chapter.chapter_num} · {countWords(chapter.content).toLocaleString()} words
							</div>
						</button>
					{/each}
				</div>
			</Panel>

			<Panel
				subtitle={data.workspace.active_novel?.title ?? 'Active novel'}
				title={activeChapter?.title ?? 'No chapter selected'}
			>
				{#if activeChapter}
					<div class="split-pane">
						<div class="split-header">
							<input
								aria-label="Search chapter"
								bind:value={searchQuery}
								class="search-field"
								placeholder="Search inside the chapter"
								type="search"
							/>
							<StatusPill
								label={hitCount > 0 ? `${hitCount} matches` : 'No active search'}
								tone={hitCount > 0 ? 'teal' : 'neutral'}
							/>
							<button
								aria-label="Reading settings"
								class="icon-button"
								onclick={() => {
									isReadingSettingsOpen = true;
								}}
								title="Reading settings"
								type="button"
							>
								<Settings2 size={16} strokeWidth={1.9} />
							</button>
						</div>
						<div
							class="reading-copy"
							style={`--reading-font-size: ${fontSizePx}px; --reading-line-height: ${lineHeightValue};`}
						>
							{#each activeParagraphs as paragraph (`${activeChapter.id}:${paragraph.slice(0, 24)}`)}
								<p>{paragraph}</p>
							{/each}
						</div>
					</div>
				{:else}
					<div class="empty-note">Không tìm thấy dữ liệu chương đang chọn.</div>
				{/if}
			</Panel>

			<div class="list-stack">
				<Panel subtitle="Observation API chưa sẵn sàng" title="Entity focus">
					<div class="empty-note">
						Chưa có extracted entity nào được persist. Bước tiếp theo là nối draft extraction vào
						observation pipeline và review queue.
					</div>
				</Panel>

				<Panel subtitle="Latest job events for context" title="Evidence panel">
					{#if data.workspace.latest_job_events.length > 0}
						<div class="detail-list">
							{#each data.workspace.latest_job_events as event (event.id)}
								<div class="evidence-card">
									<div class="status-row">
										<div class="nav-link__title">{prettyEventLabel(event.event_type)}</div>
										<StatusPill label={`#${event.sequence}`} />
									</div>
									<div class="nav-link__meta">{summarizeEventPayload(event)}</div>
								</div>
							{/each}
						</div>
					{:else}
						<div class="empty-note">
							Chưa có evidence span nào để hiển thị. Hiện panel này tạm dùng job events làm chỗ giữ
							trạng thái.
						</div>
					{/if}
				</Panel>
			</div>
		</div>
	{/if}
</div>

{#if isReadingSettingsOpen}
	<div
		aria-hidden="true"
		class="modal-backdrop"
		onclick={() => {
			isReadingSettingsOpen = false;
		}}
	></div>
	<div
		aria-labelledby="reading-settings-title"
		aria-modal="true"
		class="modal-dialog modal-dialog--compact"
		role="dialog"
	>
		<div class="modal-header">
			<div>
				<div class="eyebrow">Reading settings</div>
				<h3 id="reading-settings-title">Điều chỉnh kiểu đọc</h3>
			</div>
			<button
				aria-label="Close reading settings"
				class="icon-button"
				onclick={() => {
					isReadingSettingsOpen = false;
				}}
				type="button"
			>
				<X size={16} strokeWidth={1.9} />
			</button>
		</div>
		<div class="detail-list">
			<label class="range-field">
				<div class="status-row">
					<span class="field-label">Cỡ chữ</span>
					<strong class="range-value">{fontSizePx}px</strong>
				</div>
				<input bind:value={fontSizePx} max="22" min="13" step="1" type="range" />
			</label>

			<label class="range-field">
				<div class="status-row">
					<span class="field-label">Dãn dòng</span>
					<strong class="range-value">{lineHeightValue.toFixed(2)}</strong>
				</div>
				<input bind:value={lineHeightValue} max="2.2" min="1.35" step="0.05" type="range" />
			</label>

			<div class="callout-box">
				<div class="nav-link__title">Lưu cục bộ</div>
				<div class="nav-link__meta">
					Tùy chỉnh được lưu theo từng project trên trình duyệt hiện tại.
				</div>
			</div>

			<div class="modal-actions">
				<button class="secondary-button" onclick={resetReadingSettings} type="button">
					<RotateCcw size={16} strokeWidth={1.9} />
					Reset
				</button>
				<button
					class="action-button"
					onclick={() => {
						isReadingSettingsOpen = false;
					}}
					type="button"
				>
					Apply
				</button>
			</div>
		</div>
	</div>
{/if}

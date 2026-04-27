<script lang="ts">
	import { browser } from '$app/environment';
	import { invalidateAll } from '$app/navigation';
	import { ALargeSmall, RotateCcw, Settings, X } from 'lucide-svelte';
	import { onMount } from 'svelte';
	import Panel from '$lib/components/Panel.svelte';
	import StatusPill from '$lib/components/StatusPill.svelte';
	import type {
		StoryCharacterMention,
		StoryExtractionField,
		StoryExtractionRecord
	} from '$lib/api/types';
	import type { PageData } from './$types';

	let { data }: { data: PageData } = $props();

	const storageKey = $derived(`novelgraph:reading:${data.workspace.project.id}`);
	const settingsKey = 'novelgraph:reading-settings';
	let selectedChapterId = $state('');
	let searchQuery = $state('');
	let isReadingSettingsOpen = $state(false);
	let selectedCharacterRecordKey = $state<string | null>(null);
	let showAiHighlights = $state(true);
	let fontSizePx = $state(15);
	let lineHeightValue = $state(1.65);

	const readingSizePresets = {
		small: { fontSizePx: 14, lineHeightValue: 1.55 },
		medium: { fontSizePx: 15, lineHeightValue: 1.65 },
		large: { fontSizePx: 18, lineHeightValue: 1.85 }
	} as const;

	type ReadingSizePreset = keyof typeof readingSizePresets;

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

	onMount(() => {
		if (!browser) {
			return;
		}

		const interval = window.setInterval(() => {
			if (document.visibilityState === 'visible') {
				void invalidateAll();
			}
		}, 2000);

		return () => {
			window.clearInterval(interval);
		};
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
	const activeParagraphBlocks = $derived(
		activeChapter ? buildReadingParagraphBlocks(activeChapter.content, activeChapter.title) : []
	);
	const activeCharacterRecords = $derived(
		activeChapter
			? data.workspace.character_records.filter(
					(record) => record.chapter_num === activeChapter.chapter_num
				)
			: []
	);
	const activeHighlightMentions = $derived(buildHighlightMentions(activeCharacterRecords));
	const visibleHighlightMentions = $derived(showAiHighlights ? activeHighlightMentions : []);
	const activeReadingSizePreset = $derived(getActiveReadingSizePreset());
	const selectedCharacterRecord = $derived(
		selectedCharacterRecordKey
			? (activeCharacterRecords.find(
					(record) => characterRecordSelectionKey(record) === selectedCharacterRecordKey
				) ?? null)
			: null
	);
	const selectedCharacterAliasSummaries = $derived(
		selectedCharacterRecord
			? buildCharacterAliasSummaries(
					selectedCharacterRecord,
					data.workspace.character_records
				)
			: []
	);
	const selectedCharacterDisplayFields = $derived(
		selectedCharacterRecord ? buildCharacterDisplayFields(selectedCharacterRecord) : []
	);

	function resetReadingSettings() {
		applyReadingSizePreset('medium');
	}

	function applyReadingSizePreset(preset: ReadingSizePreset) {
		fontSizePx = readingSizePresets[preset].fontSizePx;
		lineHeightValue = readingSizePresets[preset].lineHeightValue;
	}

	function getActiveReadingSizePreset(): ReadingSizePreset | null {
		for (const preset of Object.keys(readingSizePresets) as ReadingSizePreset[]) {
			const values = readingSizePresets[preset];
			if (fontSizePx === values.fontSizePx && lineHeightValue === values.lineHeightValue) {
				return preset;
			}
		}

		return null;
	}

	type HighlightSegment = {
		text: string;
		highlighted: boolean;
		record_key: string | null;
	};

	type ReadingHighlightMention = StoryCharacterMention & {
		record_key: string;
	};

	type ParagraphBlock = {
		text: string;
		start_char: number;
		end_char: number;
	};

	function buildParagraphBlocks(text: string): ParagraphBlock[] {
		const blocks: ParagraphBlock[] = [];
		const matches = text.matchAll(/\S[\s\S]*?(?=(?:\r?\n){2,}|$)/g);

		for (const match of matches) {
			const raw = match[0];
			let start = match.index ?? 0;
			let end = start + raw.length;
			const leading = raw.match(/^\s*/)?.[0].length ?? 0;
			const trailing = raw.match(/\s*$/)?.[0].length ?? 0;
			start += leading;
			end -= trailing;

			if (end > start) {
				blocks.push({
					text: text.slice(start, end),
					start_char: start,
					end_char: end
				});
			}
		}

		return blocks;
	}

	function buildReadingParagraphBlocks(text: string, title: string): ParagraphBlock[] {
		const blocks = buildParagraphBlocks(text);
		const firstBlock = blocks[0];
		if (!firstBlock) {
			return blocks;
		}

		const normalizedTitle = normalizeReadingTitle(title);
		const normalizedFirstBlock = normalizeReadingTitle(firstBlock.text);
		if (
			normalizedFirstBlock === normalizedTitle ||
			normalizedTitle.endsWith(normalizedFirstBlock) ||
			normalizedFirstBlock.endsWith(normalizedTitle)
		) {
			return blocks.slice(1);
		}

		return blocks;
	}

	function normalizeReadingTitle(value: string) {
		return value.replace(/\s+/g, ' ').trim().toLocaleLowerCase('vi-VN');
	}

	function characterRecordIdentityKey(record: StoryExtractionRecord) {
		return record.entity_key?.trim()
			? `entity:${record.entity_key.trim().toLocaleLowerCase('en-US')}`
			: `name:${normalizeReadingTitle(record.display_name)}`;
	}

	function characterRecordSelectionKey(record: StoryExtractionRecord) {
		return `${record.chapter_num}:${characterRecordIdentityKey(record)}`;
	}

	function buildCharacterInfoMentions(mentions: StoryCharacterMention[]) {
		const seen = new Set<string>();
		const compactMentions: StoryCharacterMention[] = [];
		for (const mention of mentions) {
			const textKey = normalizeReadingTitle(mention.text);
			if (!textKey) {
				continue;
			}

			const typeKey = normalizeReadingTitle(mention.mention_type ?? '');
			const key = `${typeKey}:${textKey}`;
			if (seen.has(key)) {
				continue;
			}

			seen.add(key);
			compactMentions.push(mention);
		}

		return compactMentions.sort((left, right) =>
			left.text.localeCompare(right.text, 'vi-VN') ||
			(left.mention_type ?? '').localeCompare(right.mention_type ?? '', 'vi-VN')
		);
	}

	type CharacterAliasSummary = {
		text: string;
		alias_label: string;
		first_chapter_num: number;
		first_start_char: number;
		record_key: string;
	};

	function buildCharacterAliasSummaries(
		record: StoryExtractionRecord,
		allRecords: StoryExtractionRecord[]
	): CharacterAliasSummary[] {
		const recordIdentityKey = characterRecordIdentityKey(record);
		const displayNameKey = normalizeReadingTitle(record.display_name);
		const seen = new Set<string>();
		const summaries: CharacterAliasSummary[] = [];

		for (const field of record.fields) {
			const fieldKey = normalizeReadingTitle(field.field_key);
			if (!isCharacterAliasFieldKey(fieldKey)) {
				continue;
			}

			for (const value of field.values) {
				const text = value.value.trim();
				const textKey = normalizeReadingTitle(text);
				if (!textKey || textKey === displayNameKey) {
					continue;
				}

				const key = `${fieldKey}:${textKey}`;
				if (seen.has(key)) {
					continue;
				}

				seen.add(key);
				const firstLocation = findFirstMentionLocation(
					recordIdentityKey,
					text,
					allRecords,
					record.chapter_num
				);
				summaries.push({
					text,
					alias_label: characterAliasDisplayLabel(fieldKey, field.field_label),
					first_chapter_num: firstLocation.chapter_num,
					first_start_char: firstLocation.start_char,
					record_key: characterRecordSelectionKey(record)
				});
			}
		}

		return summaries.sort(
			(left, right) =>
				left.first_chapter_num - right.first_chapter_num ||
				left.first_start_char - right.first_start_char ||
				left.text.localeCompare(right.text, 'vi-VN')
		);
	}

	function characterAliasDisplayLabel(fieldKey: string, fieldLabel: string) {
		if (fieldKey === 'nickname' || fieldKey === 'alias' || fieldKey === 'aliases') {
			return 'Biệt danh';
		}
		if (fieldKey === 'title_or_role') {
			return 'Danh xưng';
		}
		const label = fieldLabel.trim();
		return label || 'Tên gọi khác';
	}

	function isCharacterAliasFieldKey(fieldKey: string) {
		return [
			'alias',
			'aliases',
			'other_alias',
			'other_name',
			'other_names',
			'nickname',
			'title_or_role'
		].includes(fieldKey);
	}

	function buildCharacterDisplayFields(record: StoryExtractionRecord): StoryExtractionField[] {
		return record.fields.filter((field) => {
			const fieldKey = normalizeReadingTitle(field.field_key);
			return !isCharacterAliasFieldKey(fieldKey) && field.values.length > 0;
		});
	}

	function characterDisplayFieldCount(record: StoryExtractionRecord) {
		return buildCharacterDisplayFields(record).length;
	}

	function hasEvidenceQuote(value: StoryExtractionField['values'][number]) {
		return value.evidence.some((evidence) => Boolean(evidence.quote?.trim()));
	}

	function findFirstMentionLocation(
		recordIdentityKey: string,
		text: string,
		allRecords: StoryExtractionRecord[],
		fallbackChapterNum: number
	) {
		const textKey = normalizeReadingTitle(text);
		let firstLocation: { chapter_num: number; start_char: number } | null = null;

		for (const record of allRecords) {
			if (characterRecordIdentityKey(record) !== recordIdentityKey) {
				continue;
			}

			for (const candidate of record.mentions) {
				if (normalizeReadingTitle(candidate.text) !== textKey) {
					continue;
				}

				if (
					!firstLocation ||
					record.chapter_num < firstLocation.chapter_num ||
					(record.chapter_num === firstLocation.chapter_num &&
						candidate.start_char < firstLocation.start_char)
				) {
					firstLocation = {
						chapter_num: record.chapter_num,
						start_char: candidate.start_char
					};
				}
			}
		}

		return (
			firstLocation ?? {
				chapter_num: fallbackChapterNum,
				start_char: 0
			}
		);
	}

	function openCharacterAliasSummary(alias: CharacterAliasSummary) {
		selectedCharacterRecordKey = alias.record_key;
	}

	function characterRelationshipMeta(value: {
		related_character: string | null;
		relationship_label: string | null;
	}) {
		if (!value.related_character || !value.relationship_label) {
			return '';
		}

		return `${value.relationship_label}: ${value.related_character}`;
	}

	function characterInfoMentionCount(record: StoryExtractionRecord) {
		return buildCharacterInfoMentions(record.mentions).length;
	}

	function buildHighlightMentions(records: StoryExtractionRecord[]): ReadingHighlightMention[] {
		const seen = new Set<string>();
		const mentions: ReadingHighlightMention[] = [];
		for (const record of records) {
			const recordKey = characterRecordSelectionKey(record);
			for (const mention of record.mentions) {
				const key = `${mention.start_char}:${mention.end_char}:${mention.text}`;
				if (mention.text.trim().length >= 1 && mention.end_char > mention.start_char && !seen.has(key)) {
					seen.add(key);
					mentions.push({
						...mention,
						record_key: recordKey
					});
				}
			}
		}

		return mentions.sort(
			(left, right) => left.start_char - right.start_char || right.end_char - left.end_char
		);
	}

	function highlightParagraph(
		block: ParagraphBlock,
		mentions: ReadingHighlightMention[]
	): HighlightSegment[] {
		const matches: Array<{ start: number; end: number; record_key: string }> = [];

		for (const mention of mentions) {
			if (mention.start_char < block.start_char || mention.end_char > block.end_char) {
				continue;
			}

			const start = mention.start_char - block.start_char;
			const end = mention.end_char - block.start_char;
			if (!matches.some((match) => start < match.end && end > match.start)) {
				matches.push({ start, end, record_key: mention.record_key });
			}
		}

		if (matches.length === 0) {
			return [{ text: block.text, highlighted: false, record_key: null }];
		}

		matches.sort((left, right) => left.start - right.start);

		const segments: HighlightSegment[] = [];
		let cursor = 0;
		for (const match of matches) {
			if (match.start > cursor) {
				segments.push({
					text: block.text.slice(cursor, match.start),
					highlighted: false,
					record_key: null
				});
			}
			segments.push({
				text: block.text.slice(match.start, match.end),
				highlighted: true,
				record_key: match.record_key
			});
			cursor = match.end;
		}

		if (cursor < block.text.length) {
			segments.push({
				text: block.text.slice(cursor),
				highlighted: false,
				record_key: null
			});
		}

		return segments;
	}
</script>

<div class="page-stack">
	{#if data.workspace.chapters.length === 0}
		<div class="empty-note">
			Chưa có chương nào để đọc. Hãy import truyện trước khi dùng reading workspace.
		</div>
	{:else}
		<div class="page-grid page-grid--wide reading-grid">
			<div class="reading-sticky-panel reading-chapter-list-panel">
				<Panel>
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
								</div>
							</button>
						{/each}
					</div>
				</Panel>
			</div>

			<section class="panel reading-main-panel">
				<header class="panel__header">
					<div class="panel__title">
						<h3 class="panel__heading">{activeChapter?.title ?? 'No chapter selected'}</h3>
					</div>
					{#if activeChapter}
						<div class="split-header reading-panel-controls">
							<input
								aria-label="Search chapter"
								bind:value={searchQuery}
								class="search-field"
								placeholder="Search inside the chapter"
								type="search"
							/>
							<button
								aria-label={showAiHighlights ? 'Tắt highlight AI mentions' : 'Bật highlight AI mentions'}
								aria-pressed={showAiHighlights}
								class:is-teal={showAiHighlights && activeHighlightMentions.length > 0}
								class="status-pill status-pill-button"
								disabled={activeHighlightMentions.length === 0}
								onclick={() => {
									showAiHighlights = !showAiHighlights;
								}}
								title={showAiHighlights ? 'Tắt highlight AI mentions' : 'Bật highlight AI mentions'}
								type="button"
							>
								{activeHighlightMentions.length > 0
									? `${activeHighlightMentions.length} AI mentions`
									: 'No AI highlights'}
							</button>
							<div class="reading-settings-anchor">
								<button
									aria-controls="reading-settings-popover"
									aria-expanded={isReadingSettingsOpen}
									aria-label="Reading settings"
									class="icon-button"
									onclick={() => {
										isReadingSettingsOpen = !isReadingSettingsOpen;
									}}
									title="Reading settings"
									type="button"
								>
									<Settings size={16} strokeWidth={1.9} />
								</button>
								{#if isReadingSettingsOpen}
									<div
										aria-labelledby="reading-settings-title"
										class="reading-settings-popover"
										id="reading-settings-popover"
										role="dialog"
									>
										<div class="reading-settings-popover__header">
											<h3 id="reading-settings-title">Điều chỉnh kiểu đọc</h3>
										</div>
										<div class="detail-list">
											<div aria-label="Cỡ chữ đọc" class="reading-size-preset-group" role="group">
												<button
													aria-label="Cỡ chữ nhỏ"
													class:is-active={activeReadingSizePreset === 'small'}
													class="reading-size-preset reading-size-preset--small"
													onclick={() => applyReadingSizePreset('small')}
													title="Nhỏ"
													type="button"
												>
													<ALargeSmall size={14} strokeWidth={1.9} />
												</button>
												<button
													aria-label="Cỡ chữ vừa"
													class:is-active={activeReadingSizePreset === 'medium'}
													class="reading-size-preset reading-size-preset--medium"
													onclick={() => applyReadingSizePreset('medium')}
													title="Vừa"
													type="button"
												>
													<ALargeSmall size={17} strokeWidth={1.9} />
												</button>
												<button
													aria-label="Cỡ chữ lớn"
													class:is-active={activeReadingSizePreset === 'large'}
													class="reading-size-preset reading-size-preset--large"
													onclick={() => applyReadingSizePreset('large')}
													title="Lớn"
													type="button"
												>
													<ALargeSmall size={20} strokeWidth={1.9} />
												</button>
											</div>

											<div class="reading-settings-popover__actions">
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
							</div>
						</div>
					{/if}
				</header>
				<div class="panel__content">
					{#if activeChapter}
						<div class="split-pane">
							<div
								class="reading-copy"
								style={`--reading-font-size: ${fontSizePx}px; --reading-line-height: ${lineHeightValue};`}
						>
							{#each activeParagraphBlocks as block (`${activeChapter.id}:${block.start_char}`)}
								<p>
									{#each highlightParagraph(block, visibleHighlightMentions) as segment}
										{#if segment.highlighted}
											<button
												class="reading-highlight"
												onclick={() => {
													selectedCharacterRecordKey = segment.record_key;
												}}
												type="button"
											>
												{segment.text}
											</button>
										{:else}
											{segment.text}
										{/if}
									{/each}
								</p>
							{/each}
						</div>
					</div>
					{:else}
						<div class="empty-note">Không tìm thấy dữ liệu chương đang chọn.</div>
					{/if}
				</div>
			</section>

			<div class="list-stack reading-sticky-panel reading-entity-panel">
				<Panel>
					{#if activeCharacterRecords.length > 0}
						<div class="detail-list">
							{#each activeCharacterRecords as record (record.id)}
								<button
									class:is-active={selectedCharacterRecordKey === characterRecordSelectionKey(record)}
									class="info-card character-card-button"
									onclick={() => {
										selectedCharacterRecordKey = characterRecordSelectionKey(record);
									}}
									type="button"
								>
									<div class="status-row">
										<div>
											<div class="nav-link__title">{record.display_name}</div>
											<div class="nav-link__meta">
												Chapter {record.chapter_num} · {record.entity_key ?? 'no entity key'} ·
												{characterInfoMentionCount(record)} mentions
											</div>
										</div>
										<StatusPill label={`${characterDisplayFieldCount(record)} fields`} tone="teal" />
									</div>
								</button>
							{/each}
						</div>
					{:else}
						<div class="empty-note">
							Chưa có dữ liệu nhân vật đã parse cho chương đang đọc. Hãy chạy analysis cho chương
							này trước.
						</div>
					{/if}
				</Panel>

				{#if selectedCharacterRecord}
					<dialog
						aria-labelledby="character-detail-title"
						class="character-detail-overlay"
						open
					>
						<header class="character-detail-overlay__header">
							<div class="character-detail-header-stack">
								<div class="character-detail-title-row">
									<h3 id="character-detail-title">{selectedCharacterRecord.display_name}</h3>
									<StatusPill label={selectedCharacterRecord.group_label} tone="neutral" />
								</div>
								{#if selectedCharacterAliasSummaries.length > 0}
									<div class="character-mention-chips" aria-label="Tên gọi khác">
										{#each selectedCharacterAliasSummaries as alias (`${alias.alias_label}:${alias.text}:${alias.first_chapter_num}`)}
											<button
												class="character-mention-chip"
												onclick={() => openCharacterAliasSummary(alias)}
												title={`Mở ${alias.text}`}
												type="button"
											>
												<span>{alias.text}</span>
												<span>ch.{alias.first_chapter_num} - {alias.alias_label.toLocaleLowerCase('vi-VN')}</span>
											</button>
										{/each}
									</div>
								{/if}
							</div>
							<button
								aria-label="Close character detail"
								class="icon-button"
								onclick={() => {
									selectedCharacterRecordKey = null;
								}}
								type="button"
							>
								<X size={16} strokeWidth={1.9} />
							</button>
						</header>

						<div class="character-detail-overlay__body">
							<div class="detail-list">
								<div class="status-row">
									<StatusPill label={`${selectedCharacterDisplayFields.length} fields`} tone="teal" />
								</div>

								<div class="character-detail-section">
									<div class="nav-link__title">Fields</div>
									{#if selectedCharacterDisplayFields.length > 0}
										<div class="field-stack">
											{#each selectedCharacterDisplayFields as field (field.id)}
												<div class="info-card">
													<div class="status-row">
														<div>
															<div class="nav-link__title">{field.field_label}</div>
														</div>
													</div>
													<div class="field-row__values">
														{#each field.values as value (value.id)}
															<div class="character-field-value">
																<div>{value.value}</div>
																{#if characterRelationshipMeta(value)}
																	<div class="nav-link__meta">
																		{characterRelationshipMeta(value)}
																	</div>
																{/if}
																{#if hasEvidenceQuote(value)}
																	<div class="evidence-stack">
																		{#each value.evidence as evidence, evidenceIndex (`${value.id}:${evidenceIndex}`)}
																			{#if evidence.quote}
																				<div class="nav-link__meta">
																					"{evidence.quote}"
																				</div>
																			{/if}
																		{/each}
																	</div>
																{/if}
															</div>
														{/each}
													</div>
												</div>
											{/each}
										</div>
									{:else}
										<div class="empty-note">Chưa có field nào cho nhân vật này.</div>
									{/if}
								</div>
							</div>
						</div>
					</dialog>
				{/if}
			</div>
		</div>
	{/if}
</div>

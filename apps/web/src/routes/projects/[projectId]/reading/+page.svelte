<script lang="ts">
	import { browser } from '$app/environment';
	import { invalidateAll } from '$app/navigation';
	import { ALargeSmall, ChevronDown, ChevronRight, RotateCcw, Settings, X } from 'lucide-svelte';
	import { onMount } from 'svelte';
	import Panel from '$lib/components/Panel.svelte';
	import StatusPill from '$lib/components/StatusPill.svelte';
	import { connectProjectRealtime } from '$lib/api/realtime';
	import type {
		StoryCharacterAlias,
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
	let openRelationshipEvidenceKeys = $state<Record<string, boolean>>({});

	const readingCharacterCopy = {
		relationshipsTitle: 'Quan hệ',
		evidenceSummary: 'Bằng chứng'
	} as const;

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

		return connectProjectRealtime(data.workspace.project.id, (event) => {
			if (event.event_type === 'connected') {
				return;
			}
			if (document.visibilityState === 'visible') {
				void invalidateAll();
			}
		});
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
	const visibleCharacterRecords = $derived(
		activeChapter
			? data.workspace.character_records.filter(
					(record) => record.chapter_num <= activeChapter.chapter_num
				)
			: []
	);
	const visibleRelationshipRecords = $derived(
		activeChapter
			? data.workspace.relationship_records.filter(
					(record) => record.chapter_num <= activeChapter.chapter_num
				)
			: []
	);
	const visibleCharacterAliases = $derived(
		activeChapter
			? data.workspace.character_aliases.filter(
					(alias) => alias.first_chapter_num <= activeChapter.chapter_num
				)
			: []
	);
	const activeCharacterRecords = $derived(
		activeChapter
			? visibleCharacterRecords.filter((record) => record.chapter_num === activeChapter.chapter_num)
			: []
	);
	const analysisStatusByChapterId = $derived(
		new Map(
			data.workspace.latest_analysis_chapters.map((chapter) => [chapter.chapter_id, chapter.status])
		)
	);
	const activeHighlightMentions = $derived(
		activeChapter
			? buildHighlightMentions(
					activeCharacterRecords,
					activeChapter.content,
					visibleCharacterAliases
				)
			: []
	);
	const visibleHighlightMentions = $derived(showAiHighlights ? activeHighlightMentions : []);
	const activeReadingSizePreset = $derived(getActiveReadingSizePreset());
	const selectedCharacterIdentityKey = $derived(
		selectedCharacterRecordKey ? characterSelectionIdentityKey(selectedCharacterRecordKey) : null
	);
	const selectedCharacterRecords = $derived(
		selectedCharacterIdentityKey
			? visibleCharacterRecords
					.filter((record) => characterRecordIdentityKey(record) === selectedCharacterIdentityKey)
					.sort(compareCharacterRecordsByChapter)
			: []
	);
	const selectedCharacterAliases = $derived(
		selectedCharacterIdentityKey
			? visibleCharacterAliases
					.filter((alias) => characterAliasIdentityKey(alias) === selectedCharacterIdentityKey)
					.filter((alias) => isStableReadingAliasSurface(alias.alias_text))
					.sort(compareCharacterAliasesByFirstSeen)
			: []
	);
	const selectedCharacterRecord = $derived(
		selectedCharacterIdentityKey
			? (activeCharacterRecords.find(
					(record) => characterRecordIdentityKey(record) === selectedCharacterIdentityKey
				) ??
					selectedCharacterRecords[0] ??
					null)
			: null
	);
	const selectedCharacterAliasSummaries = $derived(
		selectedCharacterRecord && selectedCharacterRecords.length > 0
			? selectedCharacterAliases.length > 0
				? buildCharacterAliasSummariesFromAliasMap(
						selectedCharacterAliases,
						selectedCharacterRecords,
						visibleCharacterRecords,
						selectedCharacterRecord
					)
				: buildCharacterAliasSummaries(
						selectedCharacterRecords,
						visibleCharacterRecords,
						selectedCharacterRecord
					)
			: []
	);
	const selectedCharacterDisplayFields = $derived(
		selectedCharacterRecords.length > 0 ? buildCharacterDisplayFields(selectedCharacterRecords) : []
	);
	const selectedCharacterRelationships = $derived(
		selectedCharacterRecord
			? buildCharacterRelationshipSummaries(
					selectedCharacterRecord,
					visibleCharacterRecords,
					visibleRelationshipRecords
				)
			: []
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

	function characterAliasIdentityKey(alias: StoryCharacterAlias) {
		return `entity:${alias.entity_key.trim().toLocaleLowerCase('en-US')}`;
	}

	function characterSelectionIdentityKey(selectionKey: string) {
		const separatorIndex = selectionKey.indexOf(':');
		return separatorIndex >= 0 ? selectionKey.slice(separatorIndex + 1) : selectionKey;
	}

	function compareCharacterRecordsByChapter(
		left: StoryExtractionRecord,
		right: StoryExtractionRecord
	) {
		return (
			left.chapter_num - right.chapter_num ||
			left.display_name.localeCompare(right.display_name, 'vi-VN') ||
			left.id.localeCompare(right.id)
		);
	}

	function chapterAnalysisStatus(chapterId: string) {
		return analysisStatusByChapterId.get(chapterId) ?? 'pending';
	}

	function chapterStatusDotClass(status: string) {
		switch (status) {
			case 'completed':
				return 'is-completed';
			case 'running':
				return 'is-running';
			case 'failed':
			case 'cancelled':
				return 'is-failed';
			case 'paused':
				return 'is-paused';
			default:
				return 'is-pending';
		}
	}

	function compareCharacterAliasesByFirstSeen(
		left: StoryCharacterAlias,
		right: StoryCharacterAlias
	) {
		return (
			left.first_chapter_num - right.first_chapter_num ||
			left.alias_text.localeCompare(right.alias_text, 'vi-VN') ||
			left.id.localeCompare(right.id)
		);
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

		return compactMentions.sort(
			(left, right) =>
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

	type CharacterRelationshipSummary = {
		related_name: string;
		label: string;
		record_key: string | null;
		evidence: Array<{
			chapter_num: number;
			quote: string;
		}>;
	};

	type InlineCharacterSegment = {
		text: string;
		record_key: string | null;
	};

	function buildCharacterAliasSummaries(
		records: StoryExtractionRecord[],
		allRecords: StoryExtractionRecord[],
		preferredRecord: StoryExtractionRecord
	): CharacterAliasSummary[] {
		const recordIdentityKey = characterRecordIdentityKey(preferredRecord);
		const displayNameKeys = new Set(
			records.map((record) => normalizeReadingTitle(record.display_name))
		);
		const seen = new Set<string>();
		const summaries: CharacterAliasSummary[] = [];
		const openRecordKey = characterRecordSelectionKey(preferredRecord);

		for (const record of records) {
			for (const field of record.fields) {
				const fieldKey = normalizeReadingTitle(field.field_key);
				if (!isCharacterAliasFieldKey(fieldKey)) {
					continue;
				}

				for (const value of field.values) {
					const text = value.value.trim();
					const textKey = normalizeReadingTitle(text);
					if (!textKey || displayNameKeys.has(textKey)) {
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
						record_key: openRecordKey
					});
				}
			}
		}

		return summaries.sort(
			(left, right) =>
				left.first_chapter_num - right.first_chapter_num ||
				left.first_start_char - right.first_start_char ||
				left.text.localeCompare(right.text, 'vi-VN')
		);
	}

	function buildCharacterAliasSummariesFromAliasMap(
		aliases: StoryCharacterAlias[],
		records: StoryExtractionRecord[],
		allRecords: StoryExtractionRecord[],
		preferredRecord: StoryExtractionRecord
	): CharacterAliasSummary[] {
		const recordIdentityKey = characterRecordIdentityKey(preferredRecord);
		const displayNameKeys = new Set(
			records.map((record) => normalizeReadingTitle(record.display_name))
		);
		const seen = new Set<string>();
		const summaries: CharacterAliasSummary[] = [];
		const openRecordKey = characterRecordSelectionKey(preferredRecord);

		for (const alias of aliases) {
			const aliasTypeKey = normalizeReadingTitle(alias.alias_type);
			if (aliasTypeKey === 'canonical_name') {
				continue;
			}

			const text = alias.alias_text.trim();
			const textKey = normalizeReadingTitle(text);
			if (!textKey || displayNameKeys.has(textKey)) {
				continue;
			}

			const key = `${aliasTypeKey}:${textKey}`;
			if (seen.has(key)) {
				continue;
			}

			seen.add(key);
			const firstLocation = findFirstMentionLocation(
				recordIdentityKey,
				text,
				allRecords,
				alias.first_chapter_num
			);
			summaries.push({
				text,
				alias_label: characterAliasDisplayLabel(aliasTypeKey, alias.alias_label),
				first_chapter_num: firstLocation.chapter_num,
				first_start_char: firstLocation.start_char,
				record_key: openRecordKey
			});
		}

		return summaries.sort(
			(left, right) =>
				left.first_chapter_num - right.first_chapter_num ||
				left.first_start_char - right.first_start_char ||
				left.text.localeCompare(right.text, 'vi-VN')
		);
	}

	function buildCharacterRelationshipSummaries(
		record: StoryExtractionRecord,
		characterRecords: StoryExtractionRecord[],
		relationshipRecords: StoryExtractionRecord[]
	): CharacterRelationshipSummary[] {
		const recordKey = record.entity_key?.trim();
		if (!recordKey) {
			return [];
		}

		const summariesByRelatedName = new Map<string, CharacterRelationshipSummary>();

		for (const relationshipRecord of relationshipRecords) {
			const pair = parseRelationshipEntityKey(relationshipRecord.entity_key);
			if (!pair || (pair.left !== recordKey && pair.right !== recordKey)) {
				continue;
			}

			const relatedKey = pair.left === recordKey ? pair.right : pair.left;
			const relatedRecord = characterRecords.find(
				(candidate) => candidate.entity_key?.trim() === relatedKey
			);
			const relatedName =
				relatedRecord?.display_name ??
				relationshipPairDisplayName(relationshipRecord, record.display_name);
			const relatedRecordKey = relatedRecord ? characterRecordSelectionKey(relatedRecord) : null;

			for (const field of relationshipRecord.fields) {
				if (normalizeReadingTitle(field.field_key) !== 'relationship') {
					continue;
				}

				for (const value of field.values) {
					if (
						value.related_character &&
						normalizeReadingTitle(value.related_character) !== normalizeReadingTitle(relatedName)
					) {
						continue;
					}

					const label = (value.relationship_label ?? value.value).trim();
					if (!label) {
						continue;
					}

					const relatedKey = normalizeReadingTitle(relatedName);
					const summary = summariesByRelatedName.get(relatedKey) ?? {
						related_name: relatedName,
						label,
						record_key: relatedRecordKey,
						evidence: []
					};
					if (!summary.record_key && relatedRecordKey) {
						summary.record_key = relatedRecordKey;
					}
					if (relationshipLabelPreferCandidate(summary.label, label)) {
						summary.label = label;
					}
					for (const evidence of relationshipEvidenceQuotes(value)) {
						pushRelationshipEvidence(summary, evidence);
					}
					summariesByRelatedName.set(relatedKey, summary);
				}
			}
		}

		return Array.from(summariesByRelatedName.values()).sort(
			(left, right) =>
				left.related_name.localeCompare(right.related_name, 'vi-VN') ||
				left.label.localeCompare(right.label, 'vi-VN')
		);
	}

	function parseRelationshipEntityKey(entityKey: string | null) {
		const parts = entityKey?.split('|') ?? [];
		if (parts.length !== 3 || parts[0] !== 'relationship') {
			return null;
		}

		return {
			left: parts[1],
			right: parts[2]
		};
	}

	function relationshipPairDisplayName(record: StoryExtractionRecord, selectedName: string) {
		const parts = record.display_name
			.split('↔')
			.map((part) => part.trim())
			.filter(Boolean);
		if (parts.length !== 2) {
			return record.display_name;
		}

		return normalizeReadingTitle(parts[0]) === normalizeReadingTitle(selectedName)
			? parts[1]
			: parts[0];
	}

	function relationshipEvidenceQuotes(value: StoryExtractionField['values'][number]) {
		return value.evidence
			.filter((evidence) => Boolean(evidence.quote?.trim()))
			.map((evidence) => ({
				chapter_num: evidence.chapter_num,
				quote: evidence.quote?.trim() ?? ''
			}))
			.sort(
				(left, right) =>
					left.chapter_num - right.chapter_num || left.quote.localeCompare(right.quote, 'vi-VN')
			);
	}

	function pushRelationshipEvidence(
		summary: CharacterRelationshipSummary,
		evidence: CharacterRelationshipSummary['evidence'][number]
	) {
		const key = `${evidence.chapter_num}:${normalizeReadingTitle(evidence.quote)}`;
		if (
			!evidence.quote ||
			summary.evidence.some(
				(existing) => `${existing.chapter_num}:${normalizeReadingTitle(existing.quote)}` === key
			)
		) {
			return;
		}

		summary.evidence.push(evidence);
		summary.evidence.sort(
			(left, right) =>
				left.chapter_num - right.chapter_num || left.quote.localeCompare(right.quote, 'vi-VN')
		);
	}

	function relationshipEvidenceKey(relationship: CharacterRelationshipSummary) {
		return `${normalizeReadingTitle(relationship.related_name)}:${normalizeReadingTitle(relationship.label)}`;
	}

	function relationshipEvidenceIsOpen(relationship: CharacterRelationshipSummary) {
		return Boolean(openRelationshipEvidenceKeys[relationshipEvidenceKey(relationship)]);
	}

	function toggleRelationshipEvidence(relationship: CharacterRelationshipSummary) {
		const key = relationshipEvidenceKey(relationship);
		openRelationshipEvidenceKeys = {
			...openRelationshipEvidenceKeys,
			[key]: !openRelationshipEvidenceKeys[key]
		};
	}

	function relationshipLabelPreferCandidate(current: string, candidate: string) {
		return relationshipLabelSpecificity(candidate) > relationshipLabelSpecificity(current);
	}

	function relationshipLabelSpecificity(value: string) {
		const tokens = normalizeReadingTitle(value)
			.replace(/[^\p{L}\p{N}]+/gu, '_')
			.split('_')
			.filter(Boolean);
		const charCount = tokens.join('').length;
		return tokens.length * 1000 + charCount;
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

	function buildCharacterDisplayFields(records: StoryExtractionRecord[]): StoryExtractionField[] {
		const fieldsByKey = new Map<string, StoryExtractionField>();
		const valueKeysByField = new Map<string, Set<string>>();

		for (const record of [...records].sort(compareCharacterRecordsByChapter)) {
			for (const field of record.fields) {
				const fieldKey = normalizeReadingTitle(field.field_key);
				if (isCharacterAliasFieldKey(fieldKey) || field.values.length === 0) {
					continue;
				}

				const aggregateKey = fieldKey || normalizeReadingTitle(field.field_label);
				let aggregateField = fieldsByKey.get(aggregateKey);
				if (!aggregateField) {
					aggregateField = {
						...field,
						id: `aggregate:${aggregateKey}`,
						values: []
					};
					fieldsByKey.set(aggregateKey, aggregateField);
					valueKeysByField.set(aggregateKey, new Set());
				}

				const seenValues = valueKeysByField.get(aggregateKey);
				for (const value of field.values) {
					const valueKey = normalizeReadingTitle(value.value);
					if (!valueKey) {
						continue;
					}

					const evidenceKey = value.evidence
						.map((evidence) => normalizeReadingTitle(evidence.quote ?? ''))
						.find(Boolean);
					const uniqueKey = `${valueKey}:${evidenceKey ?? ''}`;
					if (seenValues?.has(uniqueKey)) {
						continue;
					}

					seenValues?.add(uniqueKey);
					aggregateField.values.push(value);
				}
			}
		}

		return Array.from(fieldsByKey.values()).filter((field) => field.values.length > 0);
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

	function openCharacterSelectionKey(recordKey: string | null | undefined) {
		if (recordKey) {
			selectedCharacterRecordKey = recordKey;
		}
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

	function buildInlineCharacterSegments(
		text: string,
		records: StoryExtractionRecord[],
		aliases: StoryCharacterAlias[]
	): InlineCharacterSegment[] {
		const sourceText = text ?? '';
		if (!sourceText.trim()) {
			return [{ text: sourceText, record_key: null }];
		}

		const surfaces = buildInlineCharacterSurfaces(records, aliases);
		const matches: Array<{ start: number; end: number; text: string; record_key: string }> = [];
		const seen = new Set<string>();
		for (const surface of surfaces) {
			for (const occurrence of findReadingSurfaceOccurrences(sourceText, surface.text)) {
				const key = `${occurrence.start_char}:${occurrence.end_char}:${surface.record_key}`;
				if (seen.has(key)) {
					continue;
				}
				seen.add(key);
				matches.push({
					start: occurrence.start_char,
					end: occurrence.end_char,
					text: occurrence.text,
					record_key: surface.record_key
				});
			}
		}

		matches.sort(
			(left, right) =>
				left.start - right.start ||
				right.end - right.start - (left.end - left.start) ||
				left.text.localeCompare(right.text, 'vi-VN')
		);
		const selectedMatches: typeof matches = [];
		for (const match of matches) {
			if (!selectedMatches.some((selected) => match.start < selected.end && match.end > selected.start)) {
				selectedMatches.push(match);
			}
		}
		selectedMatches.sort((left, right) => left.start - right.start);

		const segments: InlineCharacterSegment[] = [];
		let cursor = 0;
		for (const match of selectedMatches) {
			if (match.start > cursor) {
				segments.push({
					text: sourceText.slice(cursor, match.start),
					record_key: null
				});
			}
			segments.push({
				text: sourceText.slice(match.start, match.end),
				record_key: match.record_key
			});
			cursor = match.end;
		}
		if (cursor < sourceText.length) {
			segments.push({
				text: sourceText.slice(cursor),
				record_key: null
			});
		}

		return segments.length > 0 ? segments : [{ text: sourceText, record_key: null }];
	}

	function buildInlineCharacterSurfaces(
		records: StoryExtractionRecord[],
		aliases: StoryCharacterAlias[]
	) {
		const recordByIdentity = new Map<string, StoryExtractionRecord>();
		for (const record of records) {
			const identityKey = characterRecordIdentityKey(record);
			const existing = recordByIdentity.get(identityKey);
			if (!existing || record.chapter_num >= existing.chapter_num) {
				recordByIdentity.set(identityKey, record);
			}
		}

		const surfaces: Array<{ text: string; record_key: string }> = [];
		const seen = new Set<string>();
		for (const record of recordByIdentity.values()) {
			pushInlineCharacterSurface(
				surfaces,
				seen,
				record.display_name,
				characterRecordSelectionKey(record)
			);
		}
		for (const alias of aliases) {
			if (!isStableReadingAliasSurface(alias.alias_text)) {
				continue;
			}
			const record = recordByIdentity.get(characterAliasIdentityKey(alias));
			if (!record) {
				continue;
			}
			pushInlineCharacterSurface(
				surfaces,
				seen,
				alias.alias_text,
				characterRecordSelectionKey(record)
			);
		}

		return surfaces.sort(
			(left, right) =>
				Array.from(right.text).length - Array.from(left.text).length ||
				left.text.localeCompare(right.text, 'vi-VN')
		);
	}

	function pushInlineCharacterSurface(
		surfaces: Array<{ text: string; record_key: string }>,
		seen: Set<string>,
		text: string,
		recordKey: string
	) {
		const surface = text.replace(/\s+/g, ' ').trim();
		const key = `${normalizeReadingTitle(surface)}:${recordKey}`;
		if (!surface || seen.has(key)) {
			return;
		}
		seen.add(key);
		surfaces.push({ text: surface, record_key: recordKey });
	}

	function buildHighlightMentions(
		records: StoryExtractionRecord[],
		chapterText: string,
		aliases: StoryCharacterAlias[]
	): ReadingHighlightMention[] {
		const seen = new Set<string>();
		const mentions: ReadingHighlightMention[] = [];
		const recordKeyByIdentity = new Map<string, string>();

		for (const record of records) {
			const recordKey = characterRecordSelectionKey(record);
			recordKeyByIdentity.set(characterRecordIdentityKey(record), recordKey);
			for (const mention of record.mentions) {
				const key = `${mention.start_char}:${mention.end_char}:${mention.text}`;
				if (
					mention.text.trim().length >= 1 &&
					mention.end_char > mention.start_char &&
					!seen.has(key)
				) {
					seen.add(key);
					mentions.push({
						...mention,
						record_key: recordKey
					});
				}
			}
		}

		for (const alias of aliases) {
			if (!isStableReadingAliasSurface(alias.alias_text)) {
				continue;
			}

			const identityKey = characterAliasIdentityKey(alias);
			const recordKey = recordKeyByIdentity.get(identityKey) ?? `alias:${identityKey}`;
			for (const occurrence of findReadingSurfaceOccurrences(chapterText, alias.alias_text)) {
				const key = `${occurrence.start_char}:${occurrence.end_char}:${occurrence.text}`;
				if (seen.has(key)) {
					continue;
				}

				seen.add(key);
				mentions.push({
					text: occurrence.text,
					start_char: occurrence.start_char,
					end_char: occurrence.end_char,
					mention_type: alias.alias_type,
					record_key: recordKey
				});
			}
		}

		return mentions.sort(
			(left, right) => left.start_char - right.start_char || right.end_char - left.end_char
		);
	}

	function findReadingSurfaceOccurrences(text: string, surface: string): StoryCharacterMention[] {
		const mentions: StoryCharacterMention[] = [];
		const seen = new Set<string>();
		for (const variant of readingSurfaceVariants(surface)) {
			let index = text.indexOf(variant);
			while (index >= 0) {
				const end = index + variant.length;
				if (hasReadingSurfaceBoundary(text, index, end)) {
					const key = `${index}:${end}`;
					if (!seen.has(key)) {
						seen.add(key);
						mentions.push({
							text: text.slice(index, end),
							start_char: index,
							end_char: end,
							mention_type: 'alias'
						});
					}
				}
				index = text.indexOf(variant, index + Math.max(variant.length, 1));
			}
		}

		return mentions;
	}

	function readingSurfaceVariants(surface: string) {
		const variants: string[] = [];
		const seen = new Set<string>();
		const trimmed = surface.trim();
		pushReadingSurfaceVariant(variants, seen, trimmed);

		if (trimmed.length > 0) {
			pushReadingSurfaceVariant(
				variants,
				seen,
				trimmed[0].toLocaleLowerCase('vi-VN') + trimmed.slice(1)
			);
			pushReadingSurfaceVariant(
				variants,
				seen,
				trimmed[0].toLocaleUpperCase('vi-VN') + trimmed.slice(1)
			);
		}

		return variants;
	}

	function pushReadingSurfaceVariant(variants: string[], seen: Set<string>, value: string) {
		if (value && !seen.has(value)) {
			seen.add(value);
			variants.push(value);
		}
	}

	function hasReadingSurfaceBoundary(text: string, start: number, end: number) {
		const before = start > 0 ? text[start - 1] : '';
		const after = end < text.length ? text[end] : '';

		return !isReadingWordChar(before) && !isReadingWordChar(after);
	}

	function isReadingWordChar(ch: string) {
		return Boolean(ch && /[\p{L}\p{N}_]/u.test(ch));
	}

	function isStableReadingAliasSurface(value: string) {
		const surface = value.replace(/\s+/g, ' ').trim();
		const key = normalizeReadingTitle(surface).replace(/[^\p{L}\p{N}]+/gu, '_');
		if (!surface || !key) {
			return false;
		}

		const tokens = surface.split(/\s+/);
		const charCount = Array.from(surface).filter((ch) => /[\p{L}\p{N}]/u.test(ch)).length;
		const hasUppercaseToken = tokens.some((token) => {
			const first = Array.from(token)[0];
			return Boolean(first && first === first.toLocaleUpperCase('vi-VN') && first !== first.toLocaleLowerCase('vi-VN'));
		});

		if (tokens.length === 0 || charCount < 2) {
			return false;
		}
		if (!hasUppercaseToken && tokens.length === 1 && charCount <= 4) {
			return false;
		}
		if (!hasUppercaseToken && tokens.length <= 2 && charCount <= 6) {
			return false;
		}

		return true;
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
								<div class="status-row chapter-title-row">
									<span
										aria-hidden="true"
										class={`chapter-status-dot ${chapterStatusDotClass(chapterAnalysisStatus(chapter.id))}`}
									></span>
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
								aria-label={showAiHighlights
									? 'Tắt highlight AI mentions'
									: 'Bật highlight AI mentions'}
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
												<button
													class="secondary-button"
													onclick={resetReadingSettings}
													type="button"
												>
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
														openCharacterSelectionKey(segment.record_key);
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
									class:is-active={selectedCharacterRecordKey ===
										characterRecordSelectionKey(record)}
									class="info-card character-card-button"
									onclick={() => {
										openCharacterSelectionKey(characterRecordSelectionKey(record));
									}}
									type="button"
								>
									<div class="status-row">
										<div>
											<div class="nav-link__title">{record.display_name}</div>
										</div>
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
					<dialog aria-labelledby="character-detail-title" class="character-detail-overlay" open>
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
												<span
													>ch.{alias.first_chapter_num} - {alias.alias_label.toLocaleLowerCase(
														'vi-VN'
													)}</span
												>
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
								{#if selectedCharacterDisplayFields.length > 0}
									<div class="character-detail-section">
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
																<div>
																	{#each buildInlineCharacterSegments(value.value, visibleCharacterRecords, visibleCharacterAliases) as segment, segmentIndex (`${value.id}:value:${segmentIndex}`)}
																		{#if segment.record_key}
																			<button
																				class="inline-character-link"
																				onclick={() => openCharacterSelectionKey(segment.record_key)}
																				type="button"
																			>
																				{segment.text}
																			</button>
																		{:else}
																			{segment.text}
																		{/if}
																	{/each}
																</div>
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
																					"{#each buildInlineCharacterSegments(evidence.quote, visibleCharacterRecords, visibleCharacterAliases) as segment, segmentIndex (`${value.id}:evidence:${evidenceIndex}:${segmentIndex}`)}
																						{#if segment.record_key}
																							<button
																								class="inline-character-link"
																								onclick={() => openCharacterSelectionKey(segment.record_key)}
																								type="button"
																							>
																								{segment.text}
																							</button>
																						{:else}
																							{segment.text}
																						{/if}
																					{/each}"
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
									</div>
								{/if}

								{#if selectedCharacterRelationships.length > 0}
									<div class="character-detail-section">
										<div class="field-stack">
											<div class="info-card">
												<div class="status-row">
													<div>
														<div class="nav-link__title">
															{readingCharacterCopy.relationshipsTitle}
														</div>
													</div>
												</div>
												<div class="field-row__values relationship-list">
													{#each selectedCharacterRelationships as relationship (relationship.related_name)}
														<div class="character-field-value relationship-row">
															<div class="relationship-row-main">
																<div class="relationship-summary-line">
																	{#if relationship.record_key}
																		<button
																			class="inline-character-link"
																			onclick={() => openCharacterSelectionKey(relationship.record_key)}
																			type="button"
																		>
																			{relationship.related_name}
																		</button>
																	{:else}
																		<span>{relationship.related_name}</span>
																	{/if}
																	<span class="nav-link__meta">- {relationship.label}</span>
																</div>
																{#if relationship.evidence.length > 0}
																	<button
																		aria-expanded={relationshipEvidenceIsOpen(relationship)}
																		class="relationship-evidence-toggle nav-link__meta"
																		onclick={() => toggleRelationshipEvidence(relationship)}
																		type="button"
																	>
																		{#if relationshipEvidenceIsOpen(relationship)}
																			<ChevronDown size={13} strokeWidth={1.9} />
																		{:else}
																			<ChevronRight size={13} strokeWidth={1.9} />
																		{/if}
																		<span>{readingCharacterCopy.evidenceSummary}</span>
																	</button>
																{/if}
															</div>
															{#if relationship.evidence.length > 0 && relationshipEvidenceIsOpen(relationship)}
																<div class="relationship-evidence-panel evidence-stack">
																	{#each relationship.evidence as evidence (`${evidence.chapter_num}:${evidence.quote}`)}
																		<div class="nav-link__meta">
																			ch.{evidence.chapter_num} "{#each buildInlineCharacterSegments(evidence.quote, visibleCharacterRecords, visibleCharacterAliases) as segment, segmentIndex (`${evidence.chapter_num}:${evidence.quote}:${segmentIndex}`)}
																				{#if segment.record_key}
																					<button
																						class="inline-character-link"
																						onclick={() => openCharacterSelectionKey(segment.record_key)}
																						type="button"
																					>
																						{segment.text}
																					</button>
																				{:else}
																					{segment.text}
																				{/if}
																			{/each}"
																		</div>
																	{/each}
																</div>
															{/if}
														</div>
													{/each}
												</div>
											</div>
										</div>
									</div>
								{/if}

								{#if selectedCharacterDisplayFields.length === 0 && selectedCharacterRelationships.length === 0}
									<div class="character-detail-section">
										<div class="empty-note">Chưa có field nào cho nhân vật này.</div>
									</div>
								{/if}
							</div>
						</div>
					</dialog>
				{/if}
			</div>
		</div>
	{/if}
</div>

<script lang="ts">
	import { browser } from '$app/environment';
	import { onMount } from 'svelte';
	import Panel from '$lib/components/Panel.svelte';
	import StatusPill from '$lib/components/StatusPill.svelte';
	import { chapterDocuments, chapters } from '$lib/workspace/demo';
	import type { PageData } from './$types';

	let { data }: { data: PageData } = $props();

	const storageKey = $derived(`novelgraph:reading:${data.project.id}`);

	let selectedChapterId = $state(chapters[0].id);
	let searchQuery = $state('');

	onMount(() => {
		if (!browser) {
			return;
		}

		const savedChapterId = localStorage.getItem(storageKey);
		if (savedChapterId && chapterDocuments[savedChapterId]) {
			selectedChapterId = savedChapterId;
		}
	});

	$effect(() => {
		if (browser) {
			localStorage.setItem(storageKey, selectedChapterId);
		}
	});

	const activeDocument = $derived(
		chapterDocuments[selectedChapterId] ?? chapterDocuments[chapters[0].id]
	);
	const hitCount = $derived(
		searchQuery.trim().length === 0
			? 0
			: activeDocument.paragraphs.filter((paragraph) =>
					paragraph.toLowerCase().includes(searchQuery.trim().toLowerCase())
				).length
	);
</script>

<div class="page-stack">
	<div class="page-grid page-grid--wide">
		<Panel subtitle="Persisted locally per project" title="Chapter list">
			<div class="chapter-stack">
				{#each chapters as chapter (chapter.id)}
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
							<StatusPill
								label={chapter.state}
								tone={chapter.state === 'Review' ? 'warning' : 'good'}
							/>
						</div>
						<div class="nav-link__meta">
							{chapter.words.toLocaleString()} words · {chapter.note}
						</div>
					</button>
				{/each}
			</div>
		</Panel>

		<Panel subtitle={activeDocument.location} title={activeDocument.title}>
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
				</div>
				<div class="reading-copy">
					{#each activeDocument.paragraphs as paragraph (`${activeDocument.chapterId}:${paragraph.slice(0, 24)}`)}
						<p>{paragraph}</p>
					{/each}
				</div>
			</div>
		</Panel>

		<div class="list-stack">
			<Panel subtitle="Placeholder until evidence hover is wired" title="Entity focus">
				<div class="detail-list">
					{#each activeDocument.entities as entity (entity.name)}
						<div class="info-card">
							<div class="status-row">
								<div class="nav-link__title">{entity.name}</div>
								<StatusPill label={entity.kind} />
							</div>
							{#each entity.notes as note (`${entity.name}:${note}`)}
								<div class="nav-link__meta">{note}</div>
							{/each}
						</div>
					{/each}
				</div>
			</Panel>

			<Panel subtitle="Grounding examples for the current chapter" title="Evidence panel">
				<div class="detail-list">
					{#each activeDocument.evidence as evidence (evidence.label)}
						<div class="evidence-card">
							<div class="status-row">
								<div class="nav-link__title">{evidence.label}</div>
								<StatusPill
									label={evidence.confidence}
									tone={evidence.confidence === 'High' ? 'good' : 'warning'}
								/>
							</div>
							<div class="nav-link__meta">“{evidence.quote}”</div>
						</div>
					{/each}
				</div>
			</Panel>
		</div>
	</div>
</div>

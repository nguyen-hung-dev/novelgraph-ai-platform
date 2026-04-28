<script lang="ts">
	import MetricCard from '$lib/components/MetricCard.svelte';
	import Panel from '$lib/components/Panel.svelte';
	import StatusPill from '$lib/components/StatusPill.svelte';
	import { countWords } from '$lib/workspace/presenters';
	import type { PageData } from './$types';

	let { data }: { data: PageData } = $props();

	const latestJob = $derived(data.workspace.latest_analysis_job);
</script>

<div class="page-stack">
	<section class="metrics-grid">
		<MetricCard
			detail="Source chapters currently loaded in the workspace snapshot"
			label="Chapters"
			tone="accent"
			value={data.projectView.chapterCount.toString()}
		/>
		<MetricCard
			detail="Approximate word count across the active novel"
			label="Words"
			tone="amber"
			value={data.projectView.wordCount.toLocaleString()}
		/>
		<MetricCard
			detail={data.workspace.active_novel?.title ?? 'Import a novel to populate the workspace'}
			label="Source"
			tone="teal"
			value={data.projectView.sourceLanguage}
		/>
		<MetricCard
			detail={`Updated ${data.projectView.updatedAt}`}
			label="Analysis"
			tone="rose"
			value={latestJob?.status ?? 'No job'}
		/>
	</section>

	<section class="dashboard-grid">
		<Panel subtitle="Current chapter inventory from the active novel" title="Chapter state">
			{#if data.workspace.chapters.length > 0}
				<table class="table">
					<thead>
						<tr>
							<th>Chapter</th>
							<th>Words</th>
							<th>State</th>
							<th>Note</th>
						</tr>
					</thead>
					<tbody>
						{#each data.workspace.chapters as chapter (chapter.id)}
							<tr>
								<td>{chapter.title}</td>
								<td>{countWords(chapter.content).toLocaleString()}</td>
								<td><StatusPill label="Imported" tone="good" /></td>
								<td>
									Chapter {chapter.chapter_num} is ready for reading and later evidence-first extraction.
								</td>
							</tr>
						{/each}
					</tbody>
				</table>
			{:else}
				<div class="empty-note">
					Project này chưa có chương nào. Hãy vào tab Import để nạp truyện.
				</div>
			{/if}
		</Panel>
	</section>
</div>

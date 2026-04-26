<script lang="ts">
	import type { Snippet } from 'svelte';
	import { resolve } from '$app/paths';
	import { page } from '$app/state';
	import StatusPill from '$lib/components/StatusPill.svelte';
	import { buildProjectTabs, isNavActive } from '$lib/workspace/navigation';
	import { jobStatusTone } from '$lib/workspace/presenters';
	import type { LayoutData } from './$types';

	let { data, children }: { data: LayoutData; children: Snippet } = $props();

	const tabs = $derived(buildProjectTabs(data.workspace.project.id));
	const primaryTone = $derived(
		data.workspace.latest_analysis_job
			? jobStatusTone(data.workspace.latest_analysis_job.status)
			: data.workspace.active_novel
				? 'good'
				: 'warning'
	);
</script>

<div class="page-stack">
	<section class="page-header">
		<div class="page-header__top">
			<div class="page-stack">
				<div class="eyebrow">Project</div>
				<h2>{data.projectView.name}</h2>
				<p>{data.projectView.summary}</p>
			</div>
			<div class="status-row">
				<StatusPill label={data.projectView.stage} tone={primaryTone} />
				<StatusPill label={`${data.projectView.chapterCount} chapters`} />
				<StatusPill label={data.projectView.sourceLanguage} tone="teal" />
			</div>
		</div>
		<div class="chip-row">
			{#if data.workspace.active_novel}
				<StatusPill label={data.workspace.active_novel.title} />
			{/if}
			{#if data.workspace.active_novel?.author}
				<StatusPill label={data.workspace.active_novel.author} />
			{/if}
			{#each data.projectView.tags as tag (tag)}
				<StatusPill label={tag} />
			{/each}
		</div>
	</section>

	<nav class="tab-strip">
		{#each tabs as item (item.href)}
			<a
				class:is-active={isNavActive(page.url.pathname, item)}
				class="tab-link"
				href={resolve(item.href)}
			>
				{item.label}
			</a>
		{/each}
	</nav>

	{@render children()}
</div>

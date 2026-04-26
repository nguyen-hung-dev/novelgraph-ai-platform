<script lang="ts">
	import type { Snippet } from 'svelte';
	import { resolve } from '$app/paths';
	import { page } from '$app/state';
	import StatusPill from '$lib/components/StatusPill.svelte';
	import { buildProjectTabs, isNavActive } from '$lib/workspace/navigation';
	import type { LayoutData } from './$types';

	let { data, children }: { data: LayoutData; children: Snippet } = $props();

	const tabs = $derived(buildProjectTabs(data.project.id));
</script>

<div class="page-stack">
	<section class="page-header">
		<div class="page-header__top">
			<div class="page-stack">
				<div class="eyebrow">Project</div>
				<h2>{data.project.name}</h2>
				<p>{data.project.summary}</p>
			</div>
			<div class="status-row">
				<StatusPill label={data.project.stage} tone="good" />
				<StatusPill label={`${data.project.reviewQueue} review items`} tone="warning" />
				<StatusPill label={data.project.localModel} tone="teal" />
			</div>
		</div>
		<div class="chip-row">
			{#each data.project.tags as tag (tag)}
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

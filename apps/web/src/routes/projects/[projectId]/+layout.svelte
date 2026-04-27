<script lang="ts">
	import type { Snippet } from 'svelte';
	import { resolve } from '$app/paths';
	import { page } from '$app/state';
	import { buildProjectTabs, isNavActive } from '$lib/workspace/navigation';
	import type { LayoutData } from './$types';

	let { data, children }: { data: LayoutData; children: Snippet } = $props();

	const tabs = $derived(buildProjectTabs(data.workspace.project.id));
</script>

<div class="page-stack">
	<nav class="tab-strip tab-strip--project-sticky">
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

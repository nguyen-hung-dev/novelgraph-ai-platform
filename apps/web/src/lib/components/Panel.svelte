<script lang="ts">
	import type { Snippet } from 'svelte';

	let {
		title = '',
		subtitle = '',
		children
	}: {
		title?: string;
		subtitle?: string;
		children: Snippet;
	} = $props();
</script>

<section class="panel">
	{#if title || subtitle}
		<header class="panel__header">
			<div class="panel__title">
				{#if title}
					<h3 class="panel__heading">{title}</h3>
				{/if}
				{#if subtitle}
					<p class="panel__subtitle">{subtitle}</p>
				{/if}
			</div>
		</header>
	{/if}

	<div class="panel__content">
		{@render children()}
	</div>
</section>

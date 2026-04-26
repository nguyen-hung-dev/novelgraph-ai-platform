<script lang="ts">
	let {
		label,
		value,
		detail = '',
		tone = 'neutral'
	}: {
		label: string;
		value: string;
		detail?: string;
		tone?: 'accent' | 'amber' | 'rose' | 'teal' | 'neutral';
	} = $props();
</script>

<article class={`metric-card ${tone === 'neutral' ? '' : `is-${tone}`}`}>
	<span class="metric-card__label">{label}</span>
	<strong class="metric-card__value">{value}</strong>
	{#if detail}
		<p class="metric-card__detail">{detail}</p>
	{/if}
</article>

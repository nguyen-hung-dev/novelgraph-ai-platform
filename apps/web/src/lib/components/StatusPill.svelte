<script lang="ts">
	import type { Tone } from '$lib/workspace/demo';

	let {
		label,
		tone = 'neutral'
	}: {
		label: string;
		tone?: Tone;
	} = $props();
</script>

<span class={`status-pill ${tone === 'neutral' ? '' : `is-${tone}`}`}>{label}</span>

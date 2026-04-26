<script lang="ts">
	import Panel from '$lib/components/Panel.svelte';
	import StatusPill from '$lib/components/StatusPill.svelte';
	import { reviewQueue } from '$lib/workspace/demo';

	let selectedId = $state(reviewQueue[0].id);

	const activeItem = $derived(reviewQueue.find((item) => item.id === selectedId) ?? reviewQueue[0]);
</script>

<div class="page-grid">
	<Panel subtitle="Uncertain facts stay reviewable" title="Review queue">
		<div class="review-stack">
			{#each reviewQueue as item (item.id)}
				<button
					class:is-active={selectedId === item.id}
					class="review-row"
					onclick={() => {
						selectedId = item.id;
					}}
					type="button"
				>
					<div class="status-row">
						<div class="nav-link__title">{item.title}</div>
						<StatusPill label={item.chapter} />
					</div>
					<div class="nav-link__meta">{item.summary}</div>
				</button>
			{/each}
		</div>
	</Panel>

	<Panel subtitle="Decision detail" title={activeItem.title}>
		<div class="detail-list">
			<div class="status-row">
				<StatusPill label={activeItem.chapter} />
				<StatusPill label="Needs operator judgment" tone={activeItem.severity} />
			</div>
			<div class="callout-box">
				<div class="nav-link__title">Summary</div>
				<div class="nav-link__meta">{activeItem.summary}</div>
			</div>
			<div class="evidence-card">
				<div class="nav-link__title">Evidence</div>
				<div class="nav-link__meta">“{activeItem.evidence}”</div>
			</div>
			<div class="warning-box">
				<div class="nav-link__title">Recommended handling</div>
				<div class="nav-link__meta">{activeItem.recommendation}</div>
			</div>
			<div class="table-actions">
				<button class="action-button" type="button">Accept with evidence</button>
				<button class="secondary-button" type="button">Keep unresolved</button>
				<button class="secondary-button" type="button">Escalate to glossary review</button>
			</div>
		</div>
	</Panel>
</div>

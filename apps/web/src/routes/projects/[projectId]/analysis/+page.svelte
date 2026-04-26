<script lang="ts">
	import MetricCard from '$lib/components/MetricCard.svelte';
	import Panel from '$lib/components/Panel.svelte';
	import StatusPill from '$lib/components/StatusPill.svelte';
	import { analysisEvents, analysisRun, failedChapters } from '$lib/workspace/demo';
	import type { PageData } from './$types';

	let { data }: { data: PageData } = $props();

	let runStatus = $state<'pending' | 'running' | 'cancelled' | 'completed'>('running');
	let completed = $state(analysisRun.completed);

	function startLocalRun() {
		runStatus = 'running';
		completed = Math.min(analysisRun.total, completed + 6);
	}

	function cancelLocalRun() {
		runStatus = 'cancelled';
	}
</script>

<div class="page-stack">
	<section class="metrics-grid">
		<MetricCard
			detail="Current durable job kind"
			label="Run id"
			tone="accent"
			value={analysisRun.id}
		/>
		<MetricCard
			detail="Draft extraction remains non-mutating"
			label="Provider"
			tone="teal"
			value={`${analysisRun.provider} / ${analysisRun.model}`}
		/>
		<MetricCard
			detail={`${completed} of ${analysisRun.total} chapters`}
			label="Progress"
			tone="amber"
			value={`${Math.round((completed / analysisRun.total) * 100)}%`}
		/>
		<MetricCard detail={data.project.name} label="Status" tone="rose" value={runStatus} />
	</section>

	<section class="page-grid">
		<Panel
			subtitle="Job controls should map cleanly to the Rust state machine"
			title="Analysis progress"
		>
			<div class="detail-list">
				<div class="status-row">
					<StatusPill
						label={runStatus}
						tone={runStatus === 'running' ? 'teal' : runStatus === 'cancelled' ? 'danger' : 'good'}
					/>
					<StatusPill label={analysisRun.stage} tone="good" />
				</div>
				<div class="progress-rail">
					<span style={`width: ${(completed / analysisRun.total) * 100}%`}></span>
				</div>
				<div class="table-actions">
					<button class="action-button" onclick={startLocalRun} type="button"
						>Start local extraction</button
					>
					<button class="secondary-button" onclick={cancelLocalRun} type="button">Cancel run</button
					>
					<button class="secondary-button" type="button">Retry failed chapters</button>
				</div>
				<div class="callout-box">
					<div class="nav-link__title">Current stage</div>
					<div class="nav-link__meta">
						Schema-constrained chapter extraction first, then review queue routing and later
						observation persistence.
					</div>
				</div>
			</div>
		</Panel>

		<div class="list-stack">
			<Panel subtitle="Operator-readable event trail" title="Progress events">
				<div class="event-stack">
					{#each analysisEvents as event (event)}
						<div class="event-row">
							<div class="nav-link__meta">{event}</div>
						</div>
					{/each}
				</div>
			</Panel>

			<Panel subtitle="Retry targets should stay explicit" title="Failed chapters">
				<div class="detail-list">
					{#each failedChapters as item (item.title)}
						<div class="review-row">
							<div class="nav-link__title">{item.title}</div>
							<div class="nav-link__meta">{item.reason}</div>
						</div>
					{/each}
				</div>
			</Panel>
		</div>
	</section>
</div>

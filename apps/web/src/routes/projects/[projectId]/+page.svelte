<script lang="ts">
	import MetricCard from '$lib/components/MetricCard.svelte';
	import Panel from '$lib/components/Panel.svelte';
	import StatusPill from '$lib/components/StatusPill.svelte';
	import { analysisRun, chapters, providerProfiles, reviewQueue } from '$lib/workspace/demo';
	import type { PageData } from './$types';

	let { data }: { data: PageData } = $props();
</script>

<div class="page-stack">
	<section class="metrics-grid">
		<MetricCard
			detail="Source material currently loaded in the workspace"
			label="Chapters"
			tone="accent"
			value={data.project.chapterCount.toString()}
		/>
		<MetricCard
			detail="Ready for evidence-first triage"
			label="Review queue"
			tone="amber"
			value={data.project.reviewQueue.toString()}
		/>
		<MetricCard
			detail="Primary extraction target for desktop mode"
			label="Local model"
			tone="teal"
			value={data.project.activeModel}
		/>
		<MetricCard
			detail={`Updated ${data.project.updatedAt}`}
			label="Workspace state"
			tone="rose"
			value={data.project.stage}
		/>
	</section>

	<section class="dashboard-grid">
		<Panel subtitle="Current chapter inventory" title="Chapter state">
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
					{#each chapters as chapter (chapter.id)}
						<tr>
							<td>{chapter.title}</td>
							<td>{chapter.words.toLocaleString()}</td>
							<td
								><StatusPill
									label={chapter.state}
									tone={chapter.state === 'Review' ? 'warning' : 'good'}
								/></td
							>
							<td>{chapter.note}</td>
						</tr>
					{/each}
				</tbody>
			</table>
		</Panel>

		<div class="list-stack">
			<Panel subtitle="Active job snapshot" title="Analysis queue">
				<div class="detail-list">
					<div class="status-row">
						<StatusPill label={analysisRun.status} tone="teal" />
						<StatusPill label={analysisRun.provider} />
					</div>
					<div class="progress-rail">
						<span style={`width: ${(analysisRun.completed / analysisRun.total) * 100}%`}></span>
					</div>
					<div class="nav-link__meta">
						{analysisRun.completed} / {analysisRun.total} chapters complete, queue depth {analysisRun.queueDepth}
					</div>
				</div>
			</Panel>

			<Panel subtitle="Provider layout draft" title="Execution surfaces">
				<div class="detail-list">
					{#each providerProfiles as provider (provider.name)}
						<div class="event-row">
							<div>
								<div class="nav-link__title">{provider.name}</div>
								<div class="nav-link__meta">
									{provider.mode} · {provider.model}
								</div>
							</div>
							<StatusPill
								label={provider.status}
								tone={provider.status === 'Connected' ? 'good' : 'warning'}
							/>
						</div>
					{/each}
				</div>
			</Panel>

			<Panel subtitle="Items requiring operator judgment" title="Review pressure">
				<div class="detail-list">
					{#each reviewQueue.slice(0, 2) as item (item.id)}
						<div class="review-row">
							<div class="status-row">
								<StatusPill label={item.chapter} />
								<StatusPill label={item.title} tone={item.severity} />
							</div>
							<div class="nav-link__meta">{item.summary}</div>
						</div>
					{/each}
				</div>
			</Panel>
		</div>
	</section>
</div>

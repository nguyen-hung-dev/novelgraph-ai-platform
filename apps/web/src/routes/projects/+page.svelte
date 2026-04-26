<script lang="ts">
	import { resolve } from '$app/paths';
	import MetricCard from '$lib/components/MetricCard.svelte';
	import Panel from '$lib/components/Panel.svelte';
	import StatusPill from '$lib/components/StatusPill.svelte';
	import { dashboardMetrics, projects, releaseNotes } from '$lib/workspace/demo';
</script>

<div class="page-stack">
	<section class="page-header">
		<div class="page-header__top">
			<div class="page-stack">
				<div class="eyebrow">Projects</div>
				<h2>Bookshelf and workspace entry points</h2>
				<p>
					Start from the actual product surface: open a project, inspect chapter state, and move
					straight into reading, import, analysis, or review.
				</p>
			</div>
			<div class="status-row">
				<StatusPill label="Desktop-style shell" tone="good" />
				<StatusPill label="Mock workflow state" tone="warning" />
			</div>
		</div>
	</section>

	<section class="metrics-grid">
		{#each dashboardMetrics as metric (metric.label)}
			<MetricCard
				detail={metric.detail}
				label={metric.label}
				tone={metric.tone}
				value={metric.value}
			/>
		{/each}
	</section>

	<section class="dashboard-grid">
		<div class="card-grid">
			{#each projects as project (project.id)}
				<a class="project-card" href={resolve(`/projects/${project.id}`)}>
					<div class="card-cover"></div>
					<div class="card-meta">
						<div class="status-row">
							<StatusPill label={project.stage} tone="teal" />
							<StatusPill label={`${project.reviewQueue} queued`} tone="warning" />
						</div>
						<h3 class="card-title">{project.name}</h3>
						<p class="card-copy">{project.summary}</p>
					</div>
					<div class="inline-metrics">
						<div>
							<span>Language</span>
							<strong>{project.sourceLanguage}</strong>
						</div>
						<div>
							<span>Chapters</span>
							<strong>{project.chapterCount}</strong>
						</div>
						<div>
							<span>Words</span>
							<strong>{project.wordCount.toLocaleString()}</strong>
						</div>
					</div>
					<div class="chip-row">
						{#each project.tags as tag (tag)}
							<StatusPill label={tag} />
						{/each}
					</div>
				</a>
			{/each}
		</div>

		<div class="list-stack">
			<Panel subtitle="Current UI slice" title="Foundation notes">
				<div class="detail-list">
					{#each releaseNotes as note (note.title)}
						<div class="event-row">
							<div>
								<div class="nav-link__title">{note.title}</div>
								<div class="nav-link__meta">{note.copy}</div>
							</div>
						</div>
					{/each}
				</div>
			</Panel>

			<Panel subtitle="Next wiring step" title="Backend attachment">
				<div class="callout-box">
					<div class="nav-link__title">Typed client and request tracing</div>
					<p class="panel__subtitle">
						Keep the shell stable, then replace mock data with project, chapter, and job endpoints
						from the Rust API.
					</p>
				</div>
			</Panel>
		</div>
	</section>
</div>

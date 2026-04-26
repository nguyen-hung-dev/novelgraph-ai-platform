<script lang="ts">
	import MetricCard from '$lib/components/MetricCard.svelte';
	import Panel from '$lib/components/Panel.svelte';
	import StatusPill from '$lib/components/StatusPill.svelte';
	import {
		countWords,
		formatTimestamp,
		jobStatusTone,
		prettyEventLabel,
		summarizeEventPayload
	} from '$lib/workspace/presenters';
	import type { PageData } from './$types';

	let { data }: { data: PageData } = $props();

	const latestJob = $derived(data.workspace.latest_analysis_job);
	const latestEvents = $derived(data.workspace.latest_job_events);
</script>

<div class="page-stack">
	<section class="metrics-grid">
		<MetricCard
			detail="Source chapters currently loaded in the workspace snapshot"
			label="Chapters"
			tone="accent"
			value={data.projectView.chapterCount.toString()}
		/>
		<MetricCard
			detail="Approximate word count across the active novel"
			label="Words"
			tone="amber"
			value={data.projectView.wordCount.toLocaleString()}
		/>
		<MetricCard
			detail={data.workspace.active_novel?.title ?? 'Import a novel to populate the workspace'}
			label="Source"
			tone="teal"
			value={data.projectView.sourceLanguage}
		/>
		<MetricCard
			detail={`Updated ${data.projectView.updatedAt}`}
			label="Analysis"
			tone="rose"
			value={latestJob?.status ?? 'No job'}
		/>
	</section>

	<section class="dashboard-grid">
		<Panel subtitle="Current chapter inventory from the active novel" title="Chapter state">
			{#if data.workspace.chapters.length > 0}
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
						{#each data.workspace.chapters as chapter (chapter.id)}
							<tr>
								<td>{chapter.title}</td>
								<td>{countWords(chapter.content).toLocaleString()}</td>
								<td><StatusPill label="Imported" tone="good" /></td>
								<td>
									Chapter {chapter.chapter_num} is ready for reading and later evidence-first extraction.
								</td>
							</tr>
						{/each}
					</tbody>
				</table>
			{:else}
				<div class="empty-note">
					Project này chưa có chương nào. Hãy vào tab Import để nạp truyện.
				</div>
			{/if}
		</Panel>

		<div class="list-stack">
			<Panel subtitle="Latest durable analysis job for the active novel" title="Analysis queue">
				{#if latestJob}
					<div class="detail-list">
						<div class="status-row">
							<StatusPill label={latestJob.status} tone={jobStatusTone(latestJob.status)} />
							<StatusPill label={latestJob.job_type} />
						</div>
						<div class="callout-box">
							<div class="nav-link__title">{latestJob.id}</div>
							<div class="nav-link__meta">
								Created {formatTimestamp(latestJob.created_at)} · {latestEvents.length} event(s) recorded
							</div>
						</div>
						{#if latestJob.finished_at}
							<div class="nav-link__meta">Finished {formatTimestamp(latestJob.finished_at)}</div>
						{/if}
						{#if latestJob.error_message}
							<div class="warning-box">
								<div class="nav-link__title">Last error</div>
								<div class="nav-link__meta">{latestJob.error_message}</div>
							</div>
						{/if}
					</div>
				{:else}
					<div class="empty-note">
						Chưa có analysis job nào. Xác nhận import sẽ tạo pending job đầu tiên cho truyện này.
					</div>
				{/if}
			</Panel>

			<Panel subtitle="Operator-readable trail from job_events" title="Latest events">
				{#if latestEvents.length > 0}
					<div class="detail-list">
						{#each latestEvents as event (event.id)}
							<div class="event-row">
								<div>
									<div class="nav-link__title">{prettyEventLabel(event.event_type)}</div>
									<div class="nav-link__meta">{summarizeEventPayload(event)}</div>
								</div>
								<StatusPill label={`#${event.sequence}`} />
							</div>
						{/each}
					</div>
				{:else}
					<div class="empty-note">Chưa có event nào cho project này.</div>
				{/if}
			</Panel>

			<Panel subtitle="Current execution boundary" title="Runtime notes">
				<div class="detail-list">
					<div class="event-row">
						<div>
							<div class="nav-link__title">Local-first execution</div>
							<div class="nav-link__meta">
								Workspace này ưu tiên llama.cpp local trước; cloud providers vẫn nằm sau BYOK
								boundary.
							</div>
						</div>
					</div>
					<div class="event-row">
						<div>
							<div class="nav-link__title">Observation persistence chưa bật</div>
							<div class="nav-link__meta">
								Draft extraction endpoint hiện chỉ trả về raw response và prompt metadata để đánh
								giá chất lượng.
							</div>
						</div>
					</div>
				</div>
			</Panel>
		</div>
	</section>
</div>

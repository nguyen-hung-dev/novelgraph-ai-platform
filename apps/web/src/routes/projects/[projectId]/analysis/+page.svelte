<script lang="ts">
	import { resolve } from '$app/paths';
	import MetricCard from '$lib/components/MetricCard.svelte';
	import Panel from '$lib/components/Panel.svelte';
	import StatusPill from '$lib/components/StatusPill.svelte';
	import {
		formatTimestamp,
		jobStatusTone,
		prettyEventLabel,
		summarizeEventPayload
	} from '$lib/workspace/presenters';
	import type { ActionData, PageData } from './$types';

	let { data, form }: { data: PageData; form?: ActionData } = $props();

	const latestJob = $derived(data.workspace.latest_analysis_job);
	const latestEvents = $derived(data.workspace.latest_job_events);
	const canCancel = $derived(
		latestJob ? latestJob.status === 'pending' || latestJob.status === 'running' : false
	);
	const cancelJobError = $derived(
		form?.cancelJob && 'error' in form.cancelJob ? form.cancelJob.error : null
	);
	const cancelJobOk = $derived(
		Boolean(form?.cancelJob && 'ok' in form.cancelJob && form.cancelJob.ok)
	);
</script>

<div class="page-stack">
	<section class="metrics-grid">
		<MetricCard
			detail="Latest durable analysis job for the active novel"
			label="Run id"
			tone="accent"
			value={latestJob?.id ?? 'None'}
		/>
		<MetricCard
			detail="Analysis jobs are created from the import confirm flow today"
			label="Job type"
			tone="teal"
			value={latestJob?.job_type ?? 'No job'}
		/>
		<MetricCard
			detail="Event trail currently comes from job_events"
			label="Events"
			tone="amber"
			value={latestEvents.length.toString()}
		/>
		<MetricCard
			detail={data.workspace.active_novel?.title ?? 'Import a novel first'}
			label="Status"
			tone="rose"
			value={latestJob?.status ?? 'Idle'}
		/>
	</section>

	<section class="page-grid">
		<Panel
			subtitle="Local-first orchestration attached to the Rust job state machine"
			title="Analysis job"
		>
			{#if latestJob}
				<div class="detail-list">
					<div class="status-row">
						<StatusPill label={latestJob.status} tone={jobStatusTone(latestJob.status)} />
						<StatusPill label={latestJob.job_type} />
					</div>
					<div class="callout-box">
						<div class="nav-link__title">{latestJob.id}</div>
						<div class="nav-link__meta">
							Created {formatTimestamp(latestJob.created_at)} · Updated
							{formatTimestamp(latestJob.updated_at)}
						</div>
					</div>
					{#if latestJob.finished_at}
						<div class="nav-link__meta">Finished {formatTimestamp(latestJob.finished_at)}</div>
					{/if}
					{#if latestJob.error_message}
						<div class="warning-box">
							<div class="nav-link__title">Error</div>
							<div class="nav-link__meta">{latestJob.error_message}</div>
						</div>
					{/if}
					{#if cancelJobError}
						<div class="warning-box">
							<div class="nav-link__title">Cancel job failed</div>
							<div class="nav-link__meta">{cancelJobError}</div>
						</div>
					{/if}
					{#if cancelJobOk}
						<div class="callout-box">
							<div class="nav-link__title">Cancel request accepted</div>
							<div class="nav-link__meta">
								Trang sẽ hiển thị trạng thái mới sau khi server action và load hoàn tất.
							</div>
						</div>
					{/if}
					<div class="table-actions">
						{#if canCancel}
							<form action="?/cancelJob" method="POST">
								<input name="job_id" type="hidden" value={latestJob.id} />
								<button class="secondary-button" type="submit">Cancel job</button>
							</form>
						{/if}
						<a class="toolbar-link" href={resolve('/settings/byok')}>Review provider boundary</a>
					</div>
					<div class="callout-box">
						<div class="nav-link__title">Current execution boundary</div>
						<div class="nav-link__meta">
							Job lifecycle is durable, but chapter-by-chapter worker progress and observation
							persistence are not implemented yet.
						</div>
					</div>
				</div>
			{:else}
				<div class="empty-note">
					Chưa có analysis job nào cho project này. Xác nhận import truyện sẽ tạo pending job đầu
					tiên.
				</div>
			{/if}
		</Panel>

		<div class="list-stack">
			<Panel subtitle="Operator-readable event trail" title="Progress events">
				{#if latestEvents.length > 0}
					<div class="event-stack">
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
					<div class="empty-note">Chưa có event nào cho job này.</div>
				{/if}
			</Panel>

			<Panel subtitle="Pending worker detail" title="Failure and retry">
				{#if latestJob?.error_message}
					<div class="detail-list">
						<div class="warning-box">
							<div class="nav-link__title">Job error</div>
							<div class="nav-link__meta">{latestJob.error_message}</div>
						</div>
						{#if latestJob.error_code}
							<div class="nav-link__meta">Error code: {latestJob.error_code}</div>
						{/if}
					</div>
				{:else if latestJob?.status === 'cancelled'}
					<div class="empty-note">
						Job đã bị hủy. Retry theo chapter chưa được triển khai; bước tiếp theo là thêm worker
						state machine và progress payload thực.
					</div>
				{:else}
					<div class="empty-note">
						Chưa có per-chapter failure API nào để hiển thị. Panel này sẽ dùng lại khi worker local
						bắt đầu emit progress và retry target.
					</div>
				{/if}
			</Panel>
		</div>
	</section>
</div>

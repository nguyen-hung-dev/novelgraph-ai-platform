<script lang="ts">
	import Panel from '$lib/components/Panel.svelte';
	import StatusPill from '$lib/components/StatusPill.svelte';
	import { jobStatusTone } from '$lib/workspace/presenters';
	import type { PageData } from './$types';

	let { data }: { data: PageData } = $props();
</script>

<div class="page-grid">
	<Panel subtitle="Review API is not implemented yet" title="Review queue">
		<div class="empty-note">
			Hàng đợi review sẽ xuất hiện sau khi observation persistence, uncertain fact routing và review
			item API được triển khai. Ở mốc hiện tại, UI này không còn dùng mock review queue cố định nữa.
		</div>
	</Panel>

	<Panel subtitle="Current pipeline boundary" title="Readiness">
		<div class="detail-list">
			<div class="status-row">
				<StatusPill
					label={data.workspace.latest_analysis_job?.status ?? 'No analysis job'}
					tone={data.workspace.latest_analysis_job
						? jobStatusTone(data.workspace.latest_analysis_job.status)
						: 'warning'}
				/>
				<StatusPill label={data.workspace.active_novel ? 'Novel imported' : 'Import required'} />
			</div>
			<div class="callout-box">
				<div class="nav-link__title">What is missing</div>
				<div class="nav-link__meta">
					Need observation tables, evidence span persistence, review-item generation, and operator
					decision endpoints.
				</div>
			</div>
			<div class="callout-box">
				<div class="nav-link__title">What is already live</div>
				<div class="nav-link__meta">
					Project creation, import preview/confirm, chapter reading, aggregate workspace snapshot,
					job status read, and cancel flow.
				</div>
			</div>
		</div>
	</Panel>
</div>

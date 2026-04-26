<script lang="ts">
	import Panel from '$lib/components/Panel.svelte';
	import StatusPill from '$lib/components/StatusPill.svelte';
	import { importPreviewRows, splitWarnings } from '$lib/workspace/demo';
	import type { PageData } from './$types';

	let { data }: { data: PageData } = $props();

	let isDragging = $state(false);
	let selectedFileName = $state('');
	let selectedFileSize = $state<number | null>(null);

	function applyFiles(files: FileList | null) {
		const file = files?.item(0);
		if (!file) {
			return;
		}

		selectedFileName = file.name;
		selectedFileSize = file.size;
	}
</script>

<div class="page-stack">
	<section class="page-grid">
		<Panel subtitle="TXT and Markdown entry point" title="Import novel">
			<div class="detail-list">
				<div
					aria-label="Novel import dropzone"
					class:is-active={isDragging}
					class="dropzone"
					role="group"
					ondragenter={(event) => {
						event.preventDefault();
						isDragging = true;
					}}
					ondragleave={(event) => {
						event.preventDefault();
						isDragging = false;
					}}
					ondragover={(event) => {
						event.preventDefault();
						isDragging = true;
					}}
					ondrop={(event) => {
						event.preventDefault();
						isDragging = false;
						applyFiles(event.dataTransfer?.files ?? null);
					}}
				>
					<div class="nav-link__title">Drop a source file here or pick one locally</div>
					<div class="nav-link__meta">
						Current project: {data.project.name}. File content is not persisted from the UI yet;
						this shell is preparing the workflow surface.
					</div>
					<div class="status-row">
						<label class="toolbar-link" for="novel-file">Choose file</label>
						<input
							id="novel-file"
							accept=".txt,.md,.markdown"
							hidden
							onchange={(event) => applyFiles((event.currentTarget as HTMLInputElement).files)}
							type="file"
						/>
						<StatusPill label={`${importPreviewRows.length} chapters detected`} tone="teal" />
						{#if selectedFileName}
							<StatusPill label={selectedFileName} tone="good" />
							{#if selectedFileSize !== null}
								<StatusPill label={`${Math.max(1, Math.round(selectedFileSize / 1024))} KB`} />
							{/if}
						{/if}
					</div>
				</div>

				<div class="warning-box">
					<div class="nav-link__title">Split warnings</div>
					{#each splitWarnings as warning (warning)}
						<div class="nav-link__meta">{warning}</div>
					{/each}
				</div>

				<div class="table-actions">
					<button class="action-button" type="button">Confirm import draft</button>
					<button class="secondary-button" type="button">Send preview request later</button>
				</div>
			</div>
		</Panel>

		<Panel subtitle="What the operator should verify" title="Import checks">
			<div class="detail-list">
				<div class="event-row">
					<div>
						<div class="nav-link__title">Heading detection</div>
						<div class="nav-link__meta">
							Confirm chapter boundaries before any durable persistence.
						</div>
					</div>
				</div>
				<div class="event-row">
					<div>
						<div class="nav-link__title">Source segment count</div>
						<div class="nav-link__meta">
							Keep segment structure aligned with future translation jobs.
						</div>
					</div>
				</div>
				<div class="event-row">
					<div>
						<div class="nav-link__title">Encoding and whitespace</div>
						<div class="nav-link__meta">Normalize after preview, not before operator review.</div>
					</div>
				</div>
			</div>
		</Panel>
	</section>

	<Panel subtitle="Mock preview until API wiring lands" title="Preview table">
		<table class="table">
			<thead>
				<tr>
					<th>Detected title</th>
					<th>Method</th>
					<th>Status</th>
				</tr>
			</thead>
			<tbody>
				{#each importPreviewRows as row (row.title)}
					<tr>
						<td>{row.title}</td>
						<td>{row.detection}</td>
						<td>
							<StatusPill
								label={row.status}
								tone={row.status === 'Needs review' ? 'warning' : 'good'}
							/>
						</td>
					</tr>
				{/each}
			</tbody>
		</table>
	</Panel>
</div>

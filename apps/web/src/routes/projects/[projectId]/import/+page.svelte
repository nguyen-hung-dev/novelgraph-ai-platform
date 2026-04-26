<script lang="ts">
	import Panel from '$lib/components/Panel.svelte';
	import StatusPill from '$lib/components/StatusPill.svelte';
	import type { ActionData, PageData } from './$types';

	let { data, form }: { data: PageData; form?: ActionData } = $props();

	let isDragging = $state(false);
	let selectedFileName = $state('');
	let selectedFileSize = $state<number | null>(null);
	let fileInput = $state<HTMLInputElement | null>(null);

	const importForm = $derived(
		form?.importForm ?? {
			title: '',
			author: '',
			source_language: '',
			text: ''
		}
	);
	const preview = $derived(form?.preview ?? null);
	const importFormError = $derived(
		form?.importForm && 'error' in form.importForm ? form.importForm.error : null
	);

	function applyFiles(files: FileList | null) {
		const file = files?.item(0);
		if (!file) {
			return;
		}

		if (fileInput) {
			const transfer = new DataTransfer();
			transfer.items.add(file);
			fileInput.files = transfer.files;
		}

		selectedFileName = file.name;
		selectedFileSize = file.size;
	}
</script>

<div class="page-stack">
	<section class="page-grid">
		<Panel subtitle="TXT and Markdown entry point" title="Import novel">
			<form class="detail-list" enctype="multipart/form-data" method="POST">
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
						Project hiện tại: {data.projectView.name}. Preview chỉ gọi API split chương, confirm mới
						tạo novel, chapter, source segment và analysis job.
					</div>
					<div class="status-row">
						<label class="toolbar-link" for="novel-file">Choose file</label>
						<input
							bind:this={fileInput}
							id="novel-file"
							accept=".txt,.md,.markdown"
							hidden
							name="file"
							onchange={(event) => applyFiles((event.currentTarget as HTMLInputElement).files)}
							type="file"
						/>
						<StatusPill
							label={preview ? `${preview.chapter_count} chapters detected` : 'No preview yet'}
							tone={preview ? 'teal' : 'neutral'}
						/>
						{#if selectedFileName}
							<StatusPill label={selectedFileName} tone="good" />
							{#if selectedFileSize !== null}
								<StatusPill label={`${Math.max(1, Math.round(selectedFileSize / 1024))} KB`} />
							{/if}
						{/if}
					</div>
				</div>

				<div class="form-grid">
					<label class="form-field">
						<span class="field-label">Title</span>
						<input name="title" placeholder="Tên truyện" value={importForm.title} />
					</label>

					<label class="form-field">
						<span class="field-label">Author</span>
						<input name="author" placeholder="Tác giả" value={importForm.author} />
					</label>

					<label class="form-field">
						<span class="field-label">Source language</span>
						<input
							name="source_language"
							placeholder="Ví dụ: zh, en, vi"
							value={importForm.source_language}
						/>
					</label>

					<div class="form-field">
						<span class="field-label">Current novel</span>
						<div class="callout-box">
							<div class="nav-link__title">
								{data.workspace.active_novel?.title ?? 'Chưa có truyện nào được import'}
							</div>
							<div class="nav-link__meta">
								Confirm import sẽ thêm một novel mới vào cùng project hiện tại.
							</div>
						</div>
					</div>

					<label class="form-field form-field--full">
						<span class="field-label">Source text</span>
						<textarea
							name="text"
							placeholder="Dán toàn bộ văn bản truyện vào đây nếu không dùng file."
							rows="16">{importForm.text}</textarea
						>
					</label>
				</div>

				{#if importFormError}
					<div class="warning-box">
						<div class="nav-link__title">Import validation</div>
						<div class="nav-link__meta">{importFormError}</div>
					</div>
				{/if}

				<div class="table-actions">
					<button class="secondary-button" formaction="?/preview" type="submit"
						>Preview split</button
					>
					<button class="action-button" formaction="?/confirm" type="submit">Confirm import</button>
				</div>
			</form>
		</Panel>

		<Panel subtitle="What the operator should verify" title="Import checks">
			<div class="detail-list">
				<div class="event-row">
					<div>
						<div class="nav-link__title">Heading detection</div>
						<div class="nav-link__meta">
							Kiểm tra chapter boundaries trước khi confirm để tránh tạo chapter sai.
						</div>
					</div>
				</div>
				<div class="event-row">
					<div>
						<div class="nav-link__title">Source segment count</div>
						<div class="nav-link__meta">
							Source segment là nền cho translation alignment và evidence span sau này.
						</div>
					</div>
				</div>
				<div class="event-row">
					<div>
						<div class="nav-link__title">Encoding and whitespace</div>
						<div class="nav-link__meta">
							Preview trước, rồi mới normalize nếu phát hiện heading hoặc khoảng trắng bất thường.
						</div>
					</div>
				</div>
			</div>
		</Panel>
	</section>

	<Panel subtitle="Live preview from the import preview API" title="Preview table">
		{#if preview}
			<table class="table">
				<thead>
					<tr>
						<th>Detected title</th>
						<th>Chars</th>
						<th>Range</th>
						<th>Preview</th>
					</tr>
				</thead>
				<tbody>
					{#each preview.chapters as chapter (`${chapter.chapter_num}:${chapter.title}`)}
						<tr>
							<td>{chapter.title}</td>
							<td>{chapter.char_count.toLocaleString()}</td>
							<td>
								{chapter.start_char.toLocaleString()} - {chapter.end_char.toLocaleString()}
							</td>
							<td>{chapter.preview}</td>
						</tr>
					{/each}
				</tbody>
			</table>
		{:else}
			<div class="empty-note">
				Chưa có preview nào. Hãy chạy <strong>Preview split</strong> để xem cách backend tách chương.
			</div>
		{/if}
	</Panel>
</div>

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
			genre: '',
			description: '',
			source_language: '',
			text: ''
		}
	);
	const preview = $derived(form?.preview ?? null);
	const importFormError = $derived(
		form?.importForm && 'error' in form.importForm ? form.importForm.error : null
	);
	const importFormMessage = $derived(
		form?.importForm && 'message' in form.importForm ? form.importForm.message : null
	);
	const metadataError = $derived(form && 'metadataError' in form ? form.metadataError : null);
	const metadataMessage = $derived(form && 'metadataMessage' in form ? form.metadataMessage : null);
	const activeNovel = $derived(data.workspace.active_novel);
	const languageOptions = [
		{ value: 'auto', label: 'Tự nhận diện' },
		{ value: 'zh', label: 'Tiếng Trung' },
		{ value: 'vi', label: 'Tiếng Việt' },
		{ value: 'en', label: 'Tiếng Anh' },
		{ value: 'ja', label: 'Tiếng Nhật' },
		{ value: 'ko', label: 'Tiếng Hàn' }
	];

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
						<select name="source_language">
							{#each languageOptions as option}
								<option
									value={option.value}
									selected={(importForm.source_language || 'auto') === option.value}
									>{option.label}</option
								>
							{/each}
						</select>
					</label>

					<label class="form-field">
						<span class="field-label">Genre</span>
						<input name="genre" placeholder="Thể loại" value={importForm.genre} />
					</label>

					<label class="form-field form-field--full">
						<span class="field-label">Description</span>
						<textarea
							name="description"
							placeholder="Mô tả ngắn hoặc ghi chú metadata của truyện."
							rows="3">{importForm.description}</textarea
						>
					</label>

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
				{#if importFormMessage}
					<div class="callout-box">
						<div class="nav-link__title">{importFormMessage}</div>
					</div>
				{/if}

				<div class="table-actions">
					<button class="secondary-button" formaction="?/metadata" type="submit"
						>AI fill metadata</button
					>
					<button class="secondary-button" formaction="?/preview" type="submit"
						>Preview split</button
					>
					<button class="action-button" formaction="?/confirm" type="submit">Confirm import</button>
				</div>
			</form>
		</Panel>

		<Panel subtitle="Ghi trực tiếp metadata vào DB cho truyện hiện tại" title="Current novel">
			{#if activeNovel}
				<form class="detail-list" method="POST">
					<input name="novel_id" type="hidden" value={activeNovel.id} />
					<div class="form-grid">
						<label class="form-field">
							<span class="field-label">Title</span>
							<input name="metadata_title" value={activeNovel.title} />
						</label>
						<label class="form-field">
							<span class="field-label">Author</span>
							<input name="metadata_author" value={activeNovel.author ?? ''} />
						</label>
						<label class="form-field">
							<span class="field-label">Source language</span>
							<select name="metadata_source_language">
								{#each languageOptions as option}
									<option
										value={option.value}
										selected={(activeNovel.source_language || 'auto') === option.value}
										>{option.label}</option
									>
								{/each}
							</select>
						</label>
						<label class="form-field">
							<span class="field-label">Genre</span>
							<input name="metadata_genre" value={activeNovel.genre ?? ''} />
						</label>
						<label class="form-field form-field--full">
							<span class="field-label">Description</span>
							<textarea name="metadata_description" rows="4">{activeNovel.description ?? ''}</textarea>
						</label>
					</div>

					{#if metadataError}
						<div class="warning-box">
							<div class="nav-link__title">{metadataError}</div>
						</div>
					{/if}
					{#if metadataMessage}
						<div class="callout-box">
							<div class="nav-link__title">{metadataMessage}</div>
						</div>
					{/if}

					<div class="table-actions">
						<button class="secondary-button" formaction="?/aiFillActiveMetadata" type="submit"
							>AI fill and save</button
						>
						<button class="action-button" formaction="?/saveMetadata" type="submit"
							>Save metadata</button
						>
					</div>
				</form>
			{:else}
				<div class="empty-note">Chưa có truyện nào được import trong project này.</div>
			{/if}
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

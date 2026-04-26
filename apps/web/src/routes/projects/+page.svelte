<script lang="ts">
	import { resolve } from '$app/paths';
	import { FolderOpen, RotateCcw, Trash2, X } from 'lucide-svelte';
	import MetricCard from '$lib/components/MetricCard.svelte';
	import Panel from '$lib/components/Panel.svelte';
	import StatusPill from '$lib/components/StatusPill.svelte';
	import { formatTimestamp } from '$lib/workspace/presenters';
	import type { ActionData, PageData } from './$types';

	let { data, form }: { data: PageData; form?: ActionData } = $props();

	type DeleteModalState = {
		id: string;
		name: string;
		archived: boolean;
	} | null;

	const projectCount = $derived(data.projectCards.length);
	const importedCount = $derived(
		data.projectCards.filter((project) => project.chapterCount > 0).length
	);
	const chapterCount = $derived(
		data.projectCards.reduce((total, project) => total + project.chapterCount, 0)
	);
	const wordCount = $derived(
		data.projectCards.reduce((total, project) => total + project.wordCount, 0)
	);

	let deleteModalState = $state<DeleteModalState>(null);
	let purgeProjectData = $state(false);

	function openDeleteModal(project: { id: string; name: string }, archived: boolean) {
		deleteModalState = {
			id: project.id,
			name: project.name,
			archived
		};
		purgeProjectData = archived;
	}

	function closeDeleteModal() {
		deleteModalState = null;
		purgeProjectData = false;
	}

	$effect(() => {
		if (form?.deleteProject && 'projectId' in form.deleteProject) {
			const archivedProject = data.archivedProjects.find(
				(project) => project.id === form.deleteProject.projectId
			);
			const activeProject = data.projectCards.find(
				(project) => project.id === form.deleteProject.projectId
			);
			const project = archivedProject ?? activeProject;

			if (project) {
				deleteModalState = {
					id: project.id,
					name: project.name,
					archived: Boolean(archivedProject)
				};
			}

			purgeProjectData = Boolean(form.deleteProject.purgeData);
		}
	});

	const deleteProjectError = $derived(
		form?.deleteProject &&
			'error' in form.deleteProject &&
			form.deleteProject.projectId === deleteModalState?.id
			? form.deleteProject.error
			: null
	);
	const restoreProjectError = $derived(
		form?.restoreProject && 'error' in form.restoreProject ? form.restoreProject.error : null
	);
</script>

<div class="page-stack">
	<section class="page-header">
		<div class="page-header__top">
			<div class="page-stack">
				<div class="eyebrow">Projects</div>
				<h2>Bookshelf and workspace entry points</h2>
				<p>
					Tạo project, import truyện, rồi đi thẳng vào reading hoặc analysis bằng dữ liệu thật từ
					backend.
				</p>
			</div>
			<div class="status-row">
				<StatusPill label="API-backed workspace" tone={data.apiError ? 'warning' : 'good'} />
				<StatusPill label="Create/import live" tone="teal" />
				<StatusPill
					label={`${data.archivedProjects.length} archived`}
					tone={data.archivedProjects.length > 0 ? 'warning' : 'neutral'}
				/>
			</div>
		</div>
	</section>

	<section class="metrics-grid">
		<MetricCard
			detail="Projects currently loaded from the Rust API"
			label="Projects"
			tone="accent"
			value={projectCount.toString()}
		/>
		<MetricCard
			detail="Projects that already contain an imported novel"
			label="Imported"
			tone="teal"
			value={importedCount.toString()}
		/>
		<MetricCard
			detail="Chapters available for reading and job inspection"
			label="Chapters"
			tone="amber"
			value={chapterCount.toString()}
		/>
		<MetricCard
			detail={data.apiError ?? 'Current aggregate word count across loaded projects'}
			label="Words"
			tone="rose"
			value={wordCount.toLocaleString()}
		/>
	</section>

	<section class="dashboard-grid">
		<div class="card-grid">
			{#if data.projectCards.length > 0}
				{#each data.projectCards as project (project.id)}
					<article class="project-card">
						<div class="project-card__header">
							<div class="card-cover"></div>
							<div class="project-card__actions">
								<a
									aria-label={`Open ${project.name}`}
									class="icon-button"
									href={resolve(`/projects/${project.id}`)}
									title="Open workspace"
								>
									<FolderOpen size={16} strokeWidth={1.9} />
								</a>
								<button
									aria-label={`Delete ${project.name}`}
									class="icon-button icon-button--danger"
									onclick={() => openDeleteModal(project, false)}
									title="Delete project"
									type="button"
								>
									<Trash2 size={16} strokeWidth={1.9} />
								</button>
							</div>
						</div>
						<div class="card-meta">
							<div class="status-row">
								<StatusPill label={project.stage} tone="teal" />
								<StatusPill label={`${project.chapterCount} chapters`} />
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
								<span>Words</span>
								<strong>{project.wordCount.toLocaleString()}</strong>
							</div>
							<div>
								<span>Updated</span>
								<strong>{project.updatedAt}</strong>
							</div>
						</div>
						<div class="chip-row">
							{#each project.tags as tag (tag)}
								<StatusPill label={tag} />
							{/each}
						</div>
						<div class="table-actions">
							<a class="toolbar-link" href={resolve(`/projects/${project.id}`)}>
								<FolderOpen size={16} strokeWidth={1.9} />
								Open workspace
							</a>
						</div>
					</article>
				{/each}
			{:else}
				<div class="empty-note">
					Chưa có project nào. Tạo một project mới để bắt đầu import truyện và tạo analysis job đầu
					tiên.
				</div>
			{/if}
		</div>

		<div class="list-stack">
			<Panel subtitle="Creates a real backend project row" title="New project">
				<form action="?/createProject" class="detail-list" method="POST">
					<label class="form-field">
						<span class="field-label">Project name</span>
						<input
							name="name"
							placeholder="Ví dụ: Dự án phân tích Trường Dạ"
							value={form?.createProject?.name ?? ''}
						/>
					</label>
					{#if form?.createProject?.error}
						<div class="warning-box">
							<div class="nav-link__title">Create project failed</div>
							<div class="nav-link__meta">{form.createProject.error}</div>
						</div>
					{/if}
					<div class="table-actions">
						<button class="action-button" type="submit">Create project</button>
					</div>
				</form>
			</Panel>

			<Panel subtitle="Retained locally but hidden from the bookshelf" title="Archived projects">
				{#if data.archivedProjects.length > 0}
					<div class="detail-list">
						{#each data.archivedProjects as project (project.id)}
							<div class="info-card">
								<div class="status-row">
									<div>
										<div class="nav-link__title">{project.name}</div>
										<div class="nav-link__meta">
											Last updated {formatTimestamp(project.updated_at)}
										</div>
									</div>
									<StatusPill label="Archived" tone="warning" />
								</div>
								<div class="table-actions">
									<form action="?/restoreProject" method="POST">
										<input name="project_id" type="hidden" value={project.id} />
										<button class="secondary-button" type="submit">
											<RotateCcw size={16} strokeWidth={1.9} />
											Restore
										</button>
									</form>
									<button
										class="secondary-button"
										onclick={() => openDeleteModal(project, true)}
										type="button"
									>
										<Trash2 size={16} strokeWidth={1.9} />
										Delete permanently
									</button>
								</div>
								{#if restoreProjectError && form?.restoreProject?.projectId === project.id}
									<div class="warning-box">
										<div class="nav-link__title">Restore project failed</div>
										<div class="nav-link__meta">{restoreProjectError}</div>
									</div>
								{/if}
							</div>
						{/each}
					</div>
				{:else}
					<div class="empty-note">
						Chưa có archived project nào. Khi xóa project mà không chọn purge DB, project sẽ xuất
						hiện ở đây để có thể restore.
					</div>
				{/if}
				{#if data.archivedProjectsError}
					<div class="warning-box">
						<div class="nav-link__title">Archived list failed</div>
						<div class="nav-link__meta">{data.archivedProjectsError}</div>
					</div>
				{/if}
			</Panel>

			<Panel subtitle="Current wiring status" title="Workspace slice">
				<div class="detail-list">
					<div class="event-row">
						<div>
							<div class="nav-link__title">Bookshelf is live</div>
							<div class="nav-link__meta">
								The sidebar and project cards now read from `/api/projects` and the aggregate
								workspace snapshot.
							</div>
						</div>
					</div>
					<div class="event-row">
						<div>
							<div class="nav-link__title">Archive and restore are live</div>
							<div class="nav-link__meta">
								Archive hides a project from the bookshelf while retaining DB rows; restore brings
								it back without re-importing source text.
							</div>
						</div>
					</div>
					<div class="event-row">
						<div>
							<div class="nav-link__title">Import, reading, and analysis are attached</div>
							<div class="nav-link__meta">
								Review remains a placeholder until the observation and review-item APIs exist.
							</div>
						</div>
					</div>
					{#if data.apiError}
						<div class="warning-box">
							<div class="nav-link__title">API connection</div>
							<div class="nav-link__meta">{data.apiError}</div>
						</div>
					{/if}
				</div>
			</Panel>
		</div>
	</section>
</div>

{#if deleteModalState}
	<div aria-hidden="true" class="modal-backdrop" onclick={closeDeleteModal}></div>
	<div
		aria-labelledby="delete-project-title"
		aria-modal="true"
		class="modal-dialog modal-dialog--compact"
		role="dialog"
	>
		<div class="modal-header">
			<div>
				<div class="eyebrow">
					{deleteModalState.archived ? 'Purge archived project' : 'Delete project'}
				</div>
				<h3 id="delete-project-title">
					{deleteModalState.archived ? 'Xóa vĩnh viễn' : 'Xóa'}
					{deleteModalState.name}
				</h3>
			</div>
			<button
				aria-label="Close delete dialog"
				class="icon-button"
				onclick={closeDeleteModal}
				type="button"
			>
				<X size={16} strokeWidth={1.9} />
			</button>
		</div>
		<form action="?/deleteProject" class="detail-list" method="POST">
			<input name="project_id" type="hidden" value={deleteModalState.id} />
			<div class="warning-box">
				<div class="nav-link__title">Cảnh báo</div>
				<div class="nav-link__meta">
					{#if deleteModalState.archived}
						Project này đã được archive. Nếu tiếp tục, toàn bộ novel, chapter, source segment, job
						và event liên quan sẽ bị xóa vĩnh viễn khỏi database local.
					{:else}
						Xóa project sẽ gỡ nó khỏi bookshelf. Nếu chọn xóa DB, toàn bộ novel, chapter, source
						segment, job và event của project này sẽ bị xóa khỏi database local.
					{/if}
				</div>
			</div>

			{#if deleteModalState.archived}
				<input name="purge_data" type="hidden" value="on" />
				<div class="callout-box">
					<div class="nav-link__title">Current mode</div>
					<div class="nav-link__meta">
						Purge mode: project đang archive sẽ bị xóa cứng khỏi DB và không thể restore nữa.
					</div>
				</div>
			{:else}
				<label class="toggle-row checkbox-row">
					<input
						bind:checked={purgeProjectData}
						class="checkbox"
						name="purge_data"
						type="checkbox"
					/>
					<div class="checkbox-copy">
						<div class="nav-link__title">Xóa luôn dữ liệu project trong database</div>
						<div class="nav-link__meta">
							Bỏ chọn để chỉ ẩn project khỏi bookshelf và giữ toàn bộ dữ liệu trong DB cho việc phục
							hồi sau này.
						</div>
					</div>
				</label>

				<div class="callout-box">
					<div class="nav-link__title">Current mode</div>
					<div class="nav-link__meta">
						{purgeProjectData
							? 'Purge mode: project và dữ liệu liên quan sẽ bị xóa cứng khỏi DB.'
							: 'Archive mode: project bị ẩn khỏi bookshelf, dữ liệu vẫn còn trong DB.'}
					</div>
				</div>
			{/if}

			{#if deleteProjectError}
				<div class="warning-box">
					<div class="nav-link__title">Delete project failed</div>
					<div class="nav-link__meta">{deleteProjectError}</div>
				</div>
			{/if}

			<div class="modal-actions">
				<button class="secondary-button" onclick={closeDeleteModal} type="button">Cancel</button>
				<button class="action-button action-button--danger" type="submit">
					<Trash2 size={16} strokeWidth={1.9} />
					{deleteModalState.archived ? 'Delete permanently' : 'Delete project'}
				</button>
			</div>
		</form>
	</div>
{/if}

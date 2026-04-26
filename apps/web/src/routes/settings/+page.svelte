<script lang="ts">
	import { browser } from '$app/environment';
	import { invalidateAll } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { Download, FolderSearch2, Play, RefreshCw, Server, Square } from 'lucide-svelte';
	import { onMount } from 'svelte';
	import Panel from '$lib/components/Panel.svelte';
	import StatusPill from '$lib/components/StatusPill.svelte';
	import type { ActionData, PageData } from './$types';

	let { data, form }: { data: PageData; form?: ActionData } = $props();

	const runtime = $derived(data.runtime);
	const health = $derived(data.health);
	const localRuntimeError = $derived(form?.localRuntimeAction?.error ?? null);
	const isServerStarting = $derived(Boolean(runtime?.server_running && !health?.reachable));

	onMount(() => {
		if (!browser) {
			return;
		}

		const interval = window.setInterval(() => {
			if (runtime?.server_running && !health?.reachable) {
				void invalidateAll();
			}
		}, 2500);

		return () => {
			window.clearInterval(interval);
		};
	});

	function formatByteSize(value: number | null | undefined) {
		if (value == null || Number.isNaN(value)) {
			return 'Unknown size';
		}

		const units = ['B', 'KB', 'MB', 'GB', 'TB'];
		let size = value;
		let unitIndex = 0;
		while (size >= 1024 && unitIndex < units.length - 1) {
			size /= 1024;
			unitIndex += 1;
		}

		return `${size.toFixed(unitIndex === 0 ? 0 : 2)} ${units[unitIndex]}`;
	}

	function progressLabel(bytesDownloaded: number, totalBytes: number | null) {
		if (!totalBytes || totalBytes <= 0) {
			return `${formatByteSize(bytesDownloaded)} downloaded`;
		}

		const percent = Math.min(100, Math.round((bytesDownloaded / totalBytes) * 100));
		return `${percent}% · ${formatByteSize(bytesDownloaded)} / ${formatByteSize(totalBytes)}`;
	}
</script>

<div class="page-stack">
	<section class="page-header">
		<div class="page-header__top">
			<div class="page-stack">
				<div class="eyebrow">Settings</div>
				<h2>Local LLM runtime and model library</h2>
				<p>
					Chọn GGUF có sẵn trên máy để chạy trực tiếp không copy, hoặc tải preset nhỏ về thư mục
					<code>models/</code> trong repo rồi bật bằng một nút.
				</p>
			</div>
			<div class="status-row">
				<StatusPill
					label={runtime?.server_running ? 'Server running' : 'Server stopped'}
					tone={runtime?.server_running ? 'good' : 'warning'}
				/>
				<StatusPill
					label={health?.reachable ? 'Endpoint reachable' : 'Endpoint offline'}
					tone={health?.reachable ? 'teal' : 'warning'}
				/>
				<StatusPill
					label={runtime?.selected_model
						? runtime.selected_model.source_kind === 'external'
							? 'External model'
							: 'Repo models'
						: 'No model selected'}
					tone="neutral"
				/>
			</div>
		</div>
	</section>

	<section class="settings-grid">
		<Panel subtitle="Pick, start, and stop the active GGUF" title="Local llama.cpp runtime">
			<div class="detail-list">
				<div class="info-card">
					<div class="status-row">
						<div class="nav-link__title">Server endpoint</div>
						<StatusPill label={runtime?.base_url ?? 'Unavailable'} tone="neutral" />
					</div>
					<div class="nav-link__meta">
						Alias mặc định: <code>{runtime?.default_model_alias ?? 'n/a'}</code> · binary:
						<code>{runtime?.server_binary ?? 'llama-server'}</code>
					</div>
				</div>

				<div class="info-card">
					<div class="status-row">
						<div class="nav-link__title">Current selection</div>
						<StatusPill
							label={runtime?.selected_model?.exists ? 'Ready' : 'Missing or empty'}
							tone={runtime?.selected_model?.exists ? 'good' : 'warning'}
						/>
					</div>
					{#if runtime?.selected_model}
						<div class="nav-link__meta">{runtime.selected_model.display_name}</div>
						<div class="nav-link__meta">
							<code>{runtime.selected_model.path}</code>
						</div>
						<div class="status-row">
							<StatusPill
								label={runtime.selected_model.source_kind === 'external'
									? 'Selected from local disk'
									: 'Stored in repo models'}
								tone="teal"
							/>
							<StatusPill
								label={formatByteSize(runtime.selected_model.size_bytes)}
								tone="neutral"
							/>
						</div>
					{:else}
						<div class="nav-link__meta">
							Chưa có model nào được chọn. Dùng nút chọn file để trỏ thẳng tới GGUF có sẵn trên máy.
						</div>
					{/if}
				</div>

				<div class="table-actions">
					<form action="?/pickExistingModel" method="POST">
						<button class="action-button" type="submit">
							<FolderSearch2 size={16} strokeWidth={1.9} />
							Chọn file GGUF trên máy
						</button>
					</form>
					<form action="?/startSelectedModel" method="POST">
						<button class="secondary-button" type="submit">
							<Play size={16} strokeWidth={1.9} />
							Chạy model đã chọn
						</button>
					</form>
					<form action="?/stopLocalServer" method="POST">
						<button class="secondary-button" type="submit">
							<Square size={16} strokeWidth={1.9} />
							Dừng local server
						</button>
					</form>
					<a class="secondary-button" href={resolve('/settings')}>
						<RefreshCw size={16} strokeWidth={1.9} />
						Làm mới trạng thái
					</a>
				</div>

				<div class="callout-box">
					<div class="nav-link__title">Quy tắc lưu file</div>
					<div class="nav-link__meta">
						Model chọn từ máy sẽ chạy trực tiếp theo đúng đường dẫn gốc, không copy vào repo. Preset
						tải từ UI sẽ được lưu vào <code>{runtime?.models_dir ?? 'models/'}</code>.
					</div>
				</div>

				{#if health}
					<div class="info-card">
						<div class="status-row">
							<div class="nav-link__title">Endpoint health</div>
							<StatusPill
								label={health.reachable ? 'Reachable' : 'Unreachable'}
								tone={health.reachable ? 'good' : 'warning'}
							/>
						</div>
						<div class="nav-link__meta">
							Provider <code>{health.provider}</code> · status
							<code>{health.status_code ?? 'n/a'}</code>
							{health.status_text ? ` ${health.status_text}` : ''}
						</div>
					</div>
				{/if}

				{#if isServerStarting}
					<div class="callout-box">
						<div class="nav-link__title">Local server đang khởi động</div>
						<div class="nav-link__meta">
							Process đã được start nhưng endpoint có thể cần thêm vài giây để load model và mở cổng
							`8080`. Trang sẽ tự làm mới trạng thái trong lúc chờ.
						</div>
					</div>
				{/if}

				{#if runtime?.last_error || data.runtimeError || data.healthError || localRuntimeError}
					<div class="warning-box">
						<div class="nav-link__title">Runtime note</div>
						<div class="nav-link__meta">
							{localRuntimeError ?? runtime?.last_error ?? data.runtimeError ?? data.healthError}
						</div>
					</div>
				{/if}
			</div>
		</Panel>

		<Panel subtitle="Small presets downloaded into the repo models folder" title="Preset downloads">
			<div class="detail-list">
				{#if runtime?.active_download}
					<div class="info-card">
						<div class="status-row">
							<div class="nav-link__title">{runtime.active_download.preset_name}</div>
							<StatusPill label={runtime.active_download.status} tone="warning" />
						</div>
						<div class="nav-link__meta">
							{progressLabel(
								runtime.active_download.bytes_downloaded,
								runtime.active_download.total_bytes
							)}
						</div>
						<div class="nav-link__meta">
							<code>{runtime.active_download.target_path}</code>
						</div>
						{#if runtime.active_download.error_message}
							<div class="warning-box">
								<div class="nav-link__title">Download error</div>
								<div class="nav-link__meta">{runtime.active_download.error_message}</div>
							</div>
						{/if}
					</div>
				{/if}

				{#if runtime}
					{#each runtime.presets as preset (preset.id)}
						<div class="info-card">
							<div class="status-row">
								<div>
									<div class="nav-link__title">{preset.name}</div>
									<div class="nav-link__meta">{preset.description}</div>
								</div>
								<div class="status-row">
									<StatusPill label={preset.size_label} tone="neutral" />
									<StatusPill
										label={preset.active
											? 'Active'
											: preset.installed
												? 'Installed'
												: 'Not installed'}
										tone={preset.active ? 'good' : preset.installed ? 'teal' : 'warning'}
									/>
								</div>
							</div>
							<div class="nav-link__meta">
								File <code>{preset.filename}</code>
							</div>
							<div class="nav-link__meta">
								Source <code>{preset.source_url}</code>
							</div>
							<div class="table-actions">
								<form action="?/downloadPreset" method="POST">
									<input name="preset_id" type="hidden" value={preset.id} />
									<button class="secondary-button" type="submit">
										<Download size={16} strokeWidth={1.9} />
										{preset.installed ? 'Chạy model này' : 'Tải về và chạy'}
									</button>
								</form>
							</div>
						</div>
					{/each}
				{:else}
					<div class="empty-note">Không nạp được runtime snapshot để hiển thị preset.</div>
				{/if}
			</div>
		</Panel>

		<Panel
			subtitle="Any GGUF already present in repo models can be started directly"
			title="Repo model library"
		>
			<div class="detail-list">
				{#if runtime?.downloaded_models.length}
					{#each runtime.downloaded_models as model (model.path)}
						<div class="info-card">
							<div class="status-row">
								<div>
									<div class="nav-link__title">{model.display_name}</div>
									<div class="nav-link__meta">
										<code>{model.path}</code>
									</div>
								</div>
								<div class="status-row">
									<StatusPill label={model.preset_id ? 'Preset file' : 'Manual file'} tone="teal" />
									<StatusPill label={formatByteSize(model.size_bytes)} tone="neutral" />
								</div>
							</div>
							<div class="table-actions">
								<form action="?/activateManagedModel" method="POST">
									<input name="path" type="hidden" value={model.path} />
									<button class="secondary-button" type="submit">
										<Server size={16} strokeWidth={1.9} />
										Chạy file này
									</button>
								</form>
							</div>
						</div>
					{/each}
				{:else}
					<div class="empty-note">
						Thư mục repo models chưa có GGUF nào. Hãy tải preset hoặc tự đặt file GGUF vào đó.
					</div>
				{/if}
			</div>
		</Panel>
	</section>
</div>

<script lang="ts">
	import { browser } from '$app/environment';
	import { invalidateAll } from '$app/navigation';
	import { resolve } from '$app/paths';
	import {
		CheckCircle2,
		Download,
		FolderSearch2,
		KeyRound,
		Play,
		RefreshCw,
		Save,
		Server,
		Square
	} from 'lucide-svelte';
	import { onMount } from 'svelte';
	import Panel from '$lib/components/Panel.svelte';
	import StatusPill from '$lib/components/StatusPill.svelte';
	import type { ByokProviderKeyHealth } from '$lib/api/types';
	import type { ActionData, PageData } from './$types';

	let { data, form }: { data: PageData; form?: ActionData } = $props();

	const runtime = $derived(data.runtime);
	const health = $derived(data.health);
	const byokProviders = $derived(data.byokProviders ?? []);
	const byokConfig = $derived(data.byokConfig);
	const localRuntimeError = $derived(form?.localRuntimeAction?.error ?? null);
	const byokAction = $derived(form?.byokAction ?? null);
	const isServerStarting = $derived(Boolean(runtime?.server_running && !health?.reachable));
	let provider = $state('gemini');
	let baseUrl = $state('https://generativelanguage.googleapis.com/v1beta/openai');
	let byokModel = $state('gemini-2.5-flash');
	let apiKey = $state('');
	let apiKeyDisplay = $state('');
	let byokStateInitialized = $state(false);
	const selectedByokProvider = $derived(byokProviders.find((item) => item.id === provider));
	const byokModels = $derived(selectedByokProvider?.models ?? []);
	const byokActionMessage = $derived(
		byokAction ? ('error' in byokAction ? byokAction.error : byokAction.message) : null
	);
	const byokActionHealth = $derived(getByokActionHealth(byokAction));

	$effect(() => {
		if (byokStateInitialized) {
			return;
		}

		provider = byokConfig?.provider ?? 'gemini';
		baseUrl = byokConfig?.base_url ?? 'https://generativelanguage.googleapis.com/v1beta/openai';
		byokModel = byokConfig?.model ?? 'gemini-2.5-flash';
		apiKeyDisplay = byokConfig?.api_key_masked ?? '';
		byokStateInitialized = true;
	});

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

	function maskApiKey(value: string) {
		return '*'.repeat(Math.max(8, value.length));
	}

	function handleProviderChange(event: Event) {
		const nextProvider = (event.currentTarget as HTMLSelectElement).value;
		provider = nextProvider;
		const preset = byokProviders.find((item) => item.id === nextProvider);
		if (preset) {
			baseUrl = preset.base_url;
			byokModel = preset.default_model;
		}
		apiKey = '';
		apiKeyDisplay = nextProvider === byokConfig?.provider ? (byokConfig?.api_key_masked ?? '') : '';
	}

	function handleApiKeyPaste(event: ClipboardEvent) {
		event.preventDefault();
		const value = event.clipboardData?.getData('text')?.trim() ?? '';
		apiKey = value;
		apiKeyDisplay = value ? maskApiKey(value) : '';
	}

	function handleApiKeyInput(event: Event) {
		const value = (event.currentTarget as HTMLInputElement).value;
		if (value === '') {
			apiKey = '';
			apiKeyDisplay = '';
			return;
		}

		if (apiKeyDisplay && value.startsWith(apiKeyDisplay)) {
			const appended = value.slice(apiKeyDisplay.length);
			if (appended) {
				apiKey = `${apiKey}${appended}`;
				apiKeyDisplay = maskApiKey(apiKey);
			}
			return;
		}

		if (value.split('').every((char) => char === '*')) {
			const deletedCount = Math.max(0, apiKeyDisplay.length - value.length);
			if (deletedCount > 0 && apiKey) {
				apiKey = apiKey.slice(0, Math.max(0, apiKey.length - deletedCount));
				apiKeyDisplay = apiKey ? maskApiKey(apiKey) : '';
				return;
			}
			apiKeyDisplay = value;
			return;
		}

		apiKey = value;
		apiKeyDisplay = maskApiKey(value);
	}

	function clearPendingApiKey() {
		apiKey = '';
		apiKeyDisplay = byokConfig?.provider === provider ? (byokConfig?.api_key_masked ?? '') : '';
	}

	function getByokActionHealth(action: typeof byokAction): ByokProviderKeyHealth | null {
		if (action && 'health' in action && action.health) {
			return action.health as ByokProviderKeyHealth;
		}

		return null;
	}
</script>

<div class="page-stack">
	<section class="settings-grid">
		<div class="settings-column">
			<Panel subtitle="Pick, start, and stop the active GGUF" title="Local llama.cpp runtime">
				<div class="detail-list">
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
							<StatusPill
								label={runtime?.selected_model?.exists ? 'Ready' : 'Missing or empty'}
								tone={runtime?.selected_model?.exists ? 'good' : 'warning'}
							/>
							<div class="nav-link__title">Current selection</div>
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
								Chưa có model nào được chọn. Dùng nút chọn file để trỏ thẳng tới GGUF có sẵn
								trên máy.
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
							Model chọn từ máy sẽ chạy trực tiếp theo đúng đường dẫn gốc, không copy vào repo.
							Preset tải từ UI sẽ được lưu vào <code>{runtime?.models_dir ?? 'models/'}</code>.
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
								Process đã được start nhưng endpoint có thể cần thêm vài giây để load model và mở
								cổng `8080`. Trang sẽ tự làm mới trạng thái trong lúc chờ.
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
										<StatusPill
											label={model.preset_id ? 'Preset file' : 'Manual file'}
											tone="teal"
										/>
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
		</div>

		<div class="settings-column">
			<Panel subtitle="Google Gemini OpenAI-compatible endpoint" title="BYOK settings">
				<div class="detail-list">
					<div class="status-row">
						<StatusPill
							label={byokConfig?.has_api_key ? 'Key đã lưu trong DB' : 'Chưa có key'}
							tone={byokConfig?.has_api_key ? 'good' : 'warning'}
						/>
						<StatusPill
							label={byokConfig?.last_health_status === 'valid'
								? 'Healthy key'
								: byokConfig?.last_health_status === 'invalid'
									? 'Key lỗi'
									: 'Chưa check'}
							tone={byokConfig?.last_health_status === 'valid'
								? 'teal'
								: byokConfig?.last_health_status === 'invalid'
									? 'warning'
									: 'neutral'}
						/>
					</div>

					<form class="detail-list" method="POST">
						<input name="api_key" type="hidden" value={apiKey} />

						<div class="form-grid">
							<label class="form-field">
								<span class="field-label">Provider</span>
								<select name="provider" value={provider} onchange={handleProviderChange}>
									{#each byokProviders as item (item.id)}
										<option value={item.id}>{item.name}</option>
									{/each}
								</select>
							</label>

							<label class="form-field">
								<span class="field-label">Model</span>
								{#if byokModels.length > 0}
									<select bind:value={byokModel} name="model">
										{#each byokModels as model (model)}
											<option value={model}>{model}</option>
										{/each}
									</select>
								{:else}
									<input bind:value={byokModel} name="model" placeholder="Provider model id" type="text" />
								{/if}
							</label>

							<label class="form-field form-field--full">
								<span class="field-label">Base URL</span>
								<input bind:value={baseUrl} name="base_url" placeholder="https://api.example.com/v1" type="url" />
							</label>

							<label class="form-field form-field--full">
								<span class="field-label">API key</span>
								<input
									autocomplete="off"
									inputmode="text"
									oninput={handleApiKeyInput}
									onpaste={handleApiKeyPaste}
									placeholder={byokConfig?.has_api_key ? byokConfig.api_key_masked : 'AIza...'}
									spellcheck="false"
									type="text"
									value={apiKeyDisplay}
								/>
							</label>
						</div>

						<div class="table-actions">
							<button class="action-button" formaction="?/checkByokKey" type="submit">
								<CheckCircle2 size={16} strokeWidth={1.9} />
								Check healthy key
							</button>
							<button class="secondary-button" formaction="?/saveByokSettings" type="submit">
								<Save size={16} strokeWidth={1.9} />
								Lưu BYOK
							</button>
							<button class="secondary-button" onclick={clearPendingApiKey} type="button">
								<KeyRound size={16} strokeWidth={1.9} />
								Xóa key đang nhập
							</button>
						</div>
					</form>

					{#if byokAction}
						<div class={byokAction.success ? 'info-card' : 'warning-box'}>
							<div class="nav-link__title">
								{byokAction.kind === 'checkByokKey' ? 'Kết quả check key' : 'Kết quả lưu BYOK'}
							</div>
							<div class="nav-link__meta">
								{byokActionMessage}
								{#if byokActionHealth}
									· HTTP <code>{byokActionHealth.status_code ?? 'n/a'}</code> ·
									<code>{byokActionHealth.model}</code>
								{/if}
							</div>
						</div>
					{/if}

					{#if data.byokError}
						<div class="warning-box">
							<div class="nav-link__title">BYOK note</div>
							<div class="nav-link__meta">{data.byokError}</div>
						</div>
					{/if}
				</div>
			</Panel>

		</div>
	</section>
</div>

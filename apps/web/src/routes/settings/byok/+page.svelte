<script lang="ts">
	import Panel from '$lib/components/Panel.svelte';
	import StatusPill from '$lib/components/StatusPill.svelte';

	let provider = $state('openai-compatible');
	let baseUrl = $state('https://api.example.com/v1');
	let model = $state('gpt-4.1-mini');
	let apiKey = $state('');
	let sessionOnly = $state(true);
</script>

<div class="page-grid">
	<Panel subtitle="Session-only first" title="BYOK settings">
		<div class="detail-list">
			<div class="form-grid">
				<label class="form-field">
					<span class="field-label">Provider</span>
					<select bind:value={provider}>
						<option value="openai-compatible">OpenAI-compatible</option>
						<option value="anthropic">Anthropic</option>
						<option value="deepseek">DeepSeek</option>
						<option value="gemini">Gemini</option>
						<option value="local-proxy">Local proxy</option>
					</select>
				</label>

				<label class="form-field">
					<span class="field-label">Model</span>
					<input bind:value={model} placeholder="Provider model id" type="text" />
				</label>

				<label class="form-field form-field--full">
					<span class="field-label">Base URL</span>
					<input bind:value={baseUrl} placeholder="https://api.example.com/v1" type="url" />
				</label>

				<label class="form-field form-field--full">
					<span class="field-label">API key</span>
					<input bind:value={apiKey} placeholder="sk-..." type="password" />
				</label>
			</div>

			<label class="toggle-row">
				<input bind:checked={sessionOnly} class="checkbox" type="checkbox" />
				<span>Keep key in session memory only</span>
			</label>

			<div class="table-actions">
				<button class="action-button" type="button">Validate key</button>
				<button class="secondary-button" type="button">Clear session key</button>
				<StatusPill
					label={apiKey ? 'Masked in UI' : 'No key entered'}
					tone={apiKey ? 'good' : 'warning'}
				/>
			</div>
		</div>
	</Panel>

	<Panel subtitle="Do not weaken this boundary later" title="Security notes">
		<div class="detail-list">
			<div class="security-box">
				<div class="nav-link__title">Never store keys in browser local storage</div>
				<div class="nav-link__meta">
					Session-only mode comes first. Persistent storage requires encryption at rest.
				</div>
			</div>
			<div class="warning-box">
				<div class="nav-link__title">Do not leak provider headers into prompt traces</div>
				<div class="nav-link__meta">
					Tracing and review exports must keep auth headers out of logs and UI surfaces.
				</div>
			</div>
			<div class="callout-box">
				<div class="nav-link__title">Planned next step</div>
				<div class="nav-link__meta">
					Wire this form to the Rust proxy boundary, then add validate, mask, and clear actions
					backed by safe server responses.
				</div>
			</div>
		</div>
	</Panel>
</div>

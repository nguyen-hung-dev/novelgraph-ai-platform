<script lang="ts">
	import Panel from '$lib/components/Panel.svelte';
	import StatusPill from '$lib/components/StatusPill.svelte';
	import { providerProfiles } from '$lib/workspace/demo';
</script>

<div class="page-stack">
	<section class="page-header">
		<div class="page-header__top">
			<div class="page-stack">
				<div class="eyebrow">Settings</div>
				<h2>Workspace runtime and operator defaults</h2>
				<p>
					Keep the web shell and future desktop shell visually aligned while the backend contract
					and BYOK boundary keep evolving.
				</p>
			</div>
			<div class="status-row">
				<StatusPill label="No secrets stored" tone="danger" />
				<StatusPill label="pnpm workspace" tone="good" />
			</div>
		</div>
	</section>

	<section class="settings-grid">
		<Panel subtitle="What exists today" title="Runtime surfaces">
			<div class="detail-list">
				{#each providerProfiles as provider (provider.name)}
					<div class="info-card">
						<div class="status-row">
							<div class="nav-link__title">{provider.name}</div>
							<StatusPill
								label={provider.status}
								tone={provider.status === 'Connected' ? 'good' : 'warning'}
							/>
						</div>
						<div class="nav-link__meta">{provider.baseUrl}</div>
					</div>
				{/each}
			</div>
		</Panel>

		<Panel subtitle="Next UI wiring step" title="System decisions">
			<div class="detail-list">
				<div class="event-row">
					<div>
						<div class="nav-link__title">Attach typed API client</div>
						<div class="nav-link__meta">
							Health, projects, chapters, and job events should replace mock state first.
						</div>
					</div>
				</div>
				<div class="event-row">
					<div>
						<div class="nav-link__title">Promote split panes into reusable components</div>
						<div class="nav-link__meta">
							Reading, analysis, and review pages already share the same operational shape.
						</div>
					</div>
				</div>
				<div class="event-row">
					<div>
						<div class="nav-link__title">Prepare Tauri shell reuse</div>
						<div class="nav-link__meta">
							Avoid browser-only assumptions outside the reading-position helper.
						</div>
					</div>
				</div>
			</div>
		</Panel>
	</section>
</div>

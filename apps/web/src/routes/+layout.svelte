<script lang="ts">
	import '../app.css';
	import type { Snippet } from 'svelte';
	import { resolve } from '$app/paths';
	import { page } from '$app/state';
	import {
		BookMarked,
		Cpu,
		KeyRound,
		LayoutGrid,
		RadioTower,
		Settings2,
		Sparkles
	} from 'lucide-svelte';
	import favicon from '$lib/assets/favicon.svg';
	import StatusPill from '$lib/components/StatusPill.svelte';
	import { dashboardMetrics, projects, runtimeBadges } from '$lib/workspace/demo';
	import { isNavActive, type MatchMode, type RouteHref } from '$lib/workspace/navigation';

	let { children }: { children: Snippet } = $props();

	type WorkspaceLink = {
		label: string;
		href: RouteHref;
		match: MatchMode;
		icon: typeof LayoutGrid;
		meta: string;
	};

	const workspaceLinks: WorkspaceLink[] = [
		{
			label: 'Projects',
			href: '/projects',
			match: 'exact' as const,
			icon: LayoutGrid,
			meta: 'Bookshelf and project entry points'
		},
		{
			label: 'Settings',
			href: '/settings',
			match: 'prefix' as const,
			icon: Settings2,
			meta: 'Workspace storage, desktop parity, release notes'
		},
		{
			label: 'BYOK',
			href: '/settings/byok',
			match: 'prefix' as const,
			icon: KeyRound,
			meta: 'Session key boundary and provider forms'
		}
	];
</script>

<svelte:head>
	<link rel="icon" href={favicon} />
	<title>NovelGraph AI Platform</title>
</svelte:head>

<div class="workspace-shell">
	<aside class="workspace-sidebar">
		<section class="brand-block">
			<div class="brand-row">
				<div class="brand-mark">
					<Sparkles size={18} strokeWidth={1.9} />
				</div>
				<div>
					<div class="eyebrow">Foundation UI</div>
					<div class="brand-title">NovelGraph AI Platform</div>
				</div>
			</div>
			<p class="brand-copy">
				Dense workspace shell for local-first analysis, translation review, and hosted BYOK
				expansion.
			</p>
		</section>

		<section class="nav-group">
			<div class="nav-label">
				<span>Workspace</span>
			</div>
			<div class="nav-stack">
				{#each workspaceLinks as item (item.href)}
					<a
						class:is-active={isNavActive(page.url.pathname, item)}
						class="nav-link"
						href={resolve(item.href)}
					>
						<item.icon size={16} strokeWidth={1.9} />
						<div class="nav-link__body">
							<span class="nav-link__title">{item.label}</span>
							<span class="nav-link__meta">{item.meta}</span>
						</div>
					</a>
				{/each}
			</div>
		</section>

		<section class="project-rail">
			<div class="nav-label">
				<span>Active projects</span>
				<StatusPill label={`${projects.length} loaded`} tone="teal" />
			</div>
			<div class="project-stack">
				{#each projects as project (project.id)}
					<a
						class:is-active={page.url.pathname.startsWith(`/projects/${project.id}`)}
						class="project-link"
						href={resolve(`/projects/${project.id}`)}
					>
						<BookMarked size={16} strokeWidth={1.9} />
						<div class="project-link__body">
							<span class="project-link__title">{project.name}</span>
							<span class="project-link__meta">{project.stage}</span>
						</div>
						<span class="project-link__count">{project.reviewQueue}</span>
					</a>
				{/each}
			</div>
		</section>

		<section class="sidebar-footer">
			<div class="nav-label">
				<span>Runtime</span>
			</div>
			<div class="chip-row">
				{#each runtimeBadges as badge (badge.label)}
					<StatusPill label={badge.label} tone={badge.tone} />
				{/each}
			</div>
			<div class="info-stack">
				<div class="event-row">
					<Cpu size={16} strokeWidth={1.8} />
					<div>
						<div class="nav-link__title">{dashboardMetrics[1].value} review items</div>
						<div class="nav-link__meta">Ready for queue triage and evidence checks</div>
					</div>
				</div>
				<div class="event-row">
					<RadioTower size={16} strokeWidth={1.8} />
					<div>
						<div class="nav-link__title">llama.cpp first</div>
						<div class="nav-link__meta">Hosted providers stay behind the BYOK boundary</div>
					</div>
				</div>
			</div>
		</section>
	</aside>

	<div class="workspace-body">
		<header class="workspace-topbar">
			<div class="topbar-block">
				<div class="topbar-title">
					<h1>Workspace</h1>
					<p>Desktop-style shell for project, reading, analysis, and review flows.</p>
				</div>
			</div>
			<div class="topbar-block">
				<input
					aria-label="Search workspace"
					class="search-field"
					placeholder="Search chapters, entities, review items"
					type="search"
				/>
				<div class="toolbar-stack">
					<a class="toolbar-link" href={resolve('/projects/ashen-archive/analysis')}>
						<Cpu size={16} strokeWidth={1.8} />
						Local draft run
					</a>
					<a class="toolbar-link" href={resolve('/settings/byok')}>
						<KeyRound size={16} strokeWidth={1.8} />
						Key boundary
					</a>
				</div>
			</div>
		</header>

		<main class="workspace-main">
			{@render children()}
		</main>
	</div>
</div>

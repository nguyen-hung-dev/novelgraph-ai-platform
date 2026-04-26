<script lang="ts">
	import '../app.css';
	import { browser } from '$app/environment';
	import type { Snippet } from 'svelte';
	import { onMount } from 'svelte';
	import { resolve } from '$app/paths';
	import { page } from '$app/state';
	import type { LayoutData } from './$types';
	import {
		BookMarked,
		Cpu,
		KeyRound,
		LayoutGrid,
		Monitor,
		Moon,
		RadioTower,
		Settings2,
		Sparkles,
		Sun
	} from 'lucide-svelte';
	import favicon from '$lib/assets/favicon.svg';
	import StatusPill from '$lib/components/StatusPill.svelte';
	import { isNavActive, type MatchMode, type RouteHref } from '$lib/workspace/navigation';

	let { data, children }: { data: LayoutData; children: Snippet } = $props();

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

	const totalChapterCount = $derived(
		data.projectCards.reduce((total, project) => total + project.chapterCount, 0)
	);
	const totalWordCount = $derived(
		data.projectCards.reduce((total, project) => total + project.wordCount, 0)
	);
	const firstProjectId = $derived(data.projectNav[0]?.id ?? null);

	type ColorMode = 'light' | 'dark' | 'system';

	const colorModeStorageKey = 'novelgraph:color-mode';
	let colorMode = $state<ColorMode>('system');

	function applyColorMode(mode: ColorMode) {
		const prefersDark = window.matchMedia('(prefers-color-scheme: dark)').matches;
		const resolvedMode = mode === 'system' ? (prefersDark ? 'dark' : 'light') : mode;

		document.documentElement.dataset.colorMode = resolvedMode;
		document.documentElement.style.colorScheme = resolvedMode;
	}

	function updateColorMode(mode: ColorMode) {
		colorMode = mode;
		if (!browser) {
			return;
		}

		localStorage.setItem(colorModeStorageKey, mode);
		applyColorMode(mode);
	}

	onMount(() => {
		if (!browser) {
			return;
		}

		const savedMode = localStorage.getItem(colorModeStorageKey);
		if (savedMode === 'light' || savedMode === 'dark' || savedMode === 'system') {
			colorMode = savedMode;
		}

		applyColorMode(colorMode);

		const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)');
		const handleChange = () => {
			if (colorMode === 'system') {
				applyColorMode('system');
			}
		};

		mediaQuery.addEventListener('change', handleChange);

		return () => {
			mediaQuery.removeEventListener('change', handleChange);
		};
	});
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
					<div class="eyebrow">Workspace UI</div>
					<div class="brand-title">NovelGraph AI Platform</div>
				</div>
			</div>
			<p class="brand-copy">
				Local-first workspace for novel import, chapter reading, analysis jobs, and later
				translation review.
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
				<span>Projects</span>
				<StatusPill
					label={`${data.projectNav.length} loaded`}
					tone={data.apiError ? 'warning' : 'teal'}
				/>
			</div>
			{#if data.projectNav.length > 0}
				<div class="project-stack">
					{#each data.projectNav as project (project.id)}
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
							<span class="project-link__count">{project.chapterCount} ch</span>
						</a>
					{/each}
				</div>
			{:else}
				<div class="empty-note">Chưa có project nào được nạp từ backend.</div>
			{/if}
		</section>

		<section class="sidebar-footer">
			<div class="nav-label">
				<span>Runtime</span>
			</div>
			<div class="chip-row">
				<StatusPill label="Local-first" tone="teal" />
				<StatusPill label="Rust API" tone={data.apiError ? 'warning' : 'good'} />
				<StatusPill label="SvelteKit" />
			</div>
			<div class="info-stack">
				<div class="event-row">
					<Cpu size={16} strokeWidth={1.8} />
					<div>
						<div class="nav-link__title">{totalChapterCount} chapters visible</div>
						<div class="nav-link__meta">
							{totalWordCount.toLocaleString()} words across the loaded workspace snapshot
						</div>
					</div>
				</div>
				<div class="event-row">
					<RadioTower size={16} strokeWidth={1.8} />
					<div>
						<div class="nav-link__title">
							{data.apiError ? 'API unavailable' : 'Workspace API connected'}
						</div>
						<div class="nav-link__meta">
							llama.cpp stays first; hosted providers remain behind the BYOK boundary
						</div>
					</div>
				</div>
				{#if data.apiError}
					<div class="warning-box">
						<div class="nav-link__title">Connection note</div>
						<div class="nav-link__meta">{data.apiError}</div>
					</div>
				{/if}
			</div>
		</section>
	</aside>

	<div class="workspace-body">
		<header class="workspace-topbar">
			<div class="topbar-block">
				<div class="topbar-title">
					<h1>Workspace</h1>
					<p>
						Desktop-style shell attached to live project, chapter, and job data from the Rust API.
					</p>
				</div>
			</div>
			<div class="topbar-block">
				<input
					aria-label="Search workspace"
					class="search-field"
					placeholder="Search chapters, jobs, and future review items"
					type="search"
				/>
				<div class="toolbar-stack">
					<div aria-label="Color mode" class="icon-toggle-group" role="group">
						<button
							aria-label="Light mode"
							class:is-active={colorMode === 'light'}
							class="icon-button"
							onclick={() => updateColorMode('light')}
							title="Light mode"
							type="button"
						>
							<Sun size={16} strokeWidth={1.9} />
						</button>
						<button
							aria-label="Dark mode"
							class:is-active={colorMode === 'dark'}
							class="icon-button"
							onclick={() => updateColorMode('dark')}
							title="Dark mode"
							type="button"
						>
							<Moon size={16} strokeWidth={1.9} />
						</button>
						<button
							aria-label="System mode"
							class:is-active={colorMode === 'system'}
							class="icon-button"
							onclick={() => updateColorMode('system')}
							title="System mode"
							type="button"
						>
							<Monitor size={16} strokeWidth={1.9} />
						</button>
					</div>
					<a
						class="toolbar-link"
						href={firstProjectId
							? resolve('/projects/[projectId]/analysis', { projectId: firstProjectId })
							: resolve('/projects')}
					>
						<Cpu size={16} strokeWidth={1.8} />
						Local job view
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

<script lang="ts">
	import '../app.css';
	import { browser } from '$app/environment';
	import type { Snippet } from 'svelte';
	import { onMount } from 'svelte';
	import { resolve } from '$app/paths';
	import { page } from '$app/state';
	import type { LayoutData } from './$types';
	import {
		LayoutGrid,
		Monitor,
		Moon,
		PanelLeftClose,
		PanelLeftOpen,
		Settings,
		Sparkles,
		Sun
	} from 'lucide-svelte';
	import favicon from '$lib/assets/favicon.svg';
	import { isNavActive, type MatchMode, type RouteHref } from '$lib/workspace/navigation';

	let { data, children }: { data: LayoutData; children: Snippet } = $props();

	type WorkspaceLink = {
		label: string;
		href: RouteHref;
		match: MatchMode;
		icon: typeof LayoutGrid;
	};

	const workspaceLinks: WorkspaceLink[] = [
		{
			label: 'Projects',
			href: '/projects',
			match: 'exact' as const,
			icon: LayoutGrid
		}
	];
	const settingsLink: WorkspaceLink = {
		label: 'Settings',
		href: '/settings',
		match: 'exact' as const,
		icon: Settings
	};

	type ColorMode = 'light' | 'dark' | 'system';

	const colorModeStorageKey = 'novelgraph:color-mode';
	const sidebarCollapsedStorageKey = 'novelgraph:sidebar-collapsed';
	let colorMode = $state<ColorMode>('system');
	let isSidebarCollapsed = $state(false);

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

	function rotateColorMode() {
		const nextMode: ColorMode =
			colorMode === 'system' ? 'light' : colorMode === 'light' ? 'dark' : 'system';
		updateColorMode(nextMode);
	}

	function updateSidebarCollapsed(collapsed: boolean) {
		isSidebarCollapsed = collapsed;
		if (!browser) {
			return;
		}

		localStorage.setItem(sidebarCollapsedStorageKey, String(collapsed));
	}

	onMount(() => {
		if (!browser) {
			return;
		}

		const savedMode = localStorage.getItem(colorModeStorageKey);
		if (savedMode === 'light' || savedMode === 'dark' || savedMode === 'system') {
			colorMode = savedMode;
		}

		isSidebarCollapsed = localStorage.getItem(sidebarCollapsedStorageKey) === 'true';
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

<div class:workspace-shell--sidebar-collapsed={isSidebarCollapsed} class="workspace-shell">
	<aside aria-label="Workspace sidebar" class="workspace-sidebar">
		<section class="brand-block">
			<div class="brand-row">
				<div class="brand-identity">
					<div class="brand-mark">
						<Sparkles size={18} strokeWidth={1.9} />
					</div>
					<div class="brand-text">
						<div class="eyebrow">Workspace UI</div>
						<div class="brand-title">NovelGraph AI Platform</div>
					</div>
				</div>
				<button
					aria-label={isSidebarCollapsed ? 'Expand sidebar' : 'Collapse sidebar'}
					class="icon-button sidebar-toggle"
					onclick={() => updateSidebarCollapsed(!isSidebarCollapsed)}
					title={isSidebarCollapsed ? 'Expand sidebar' : 'Collapse sidebar'}
					type="button"
				>
					{#if isSidebarCollapsed}
						<PanelLeftOpen size={16} strokeWidth={1.9} />
					{:else}
						<PanelLeftClose size={16} strokeWidth={1.9} />
					{/if}
				</button>
			</div>
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
						title={item.label}
					>
						<item.icon size={16} strokeWidth={1.9} />
						<div class="nav-link__body">
							<span class="nav-link__title">{item.label}</span>
						</div>
					</a>
				{/each}
			</div>
		</section>

		<a
			class:is-active={isNavActive(page.url.pathname, settingsLink)}
			class="nav-link sidebar-settings-link"
			href={resolve(settingsLink.href)}
			title={settingsLink.label}
		>
			<settingsLink.icon size={16} strokeWidth={1.9} />
			<div class="nav-link__body">
				<span class="nav-link__title">{settingsLink.label}</span>
			</div>
		</a>
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
					<button
						aria-label={`Color mode: ${colorMode}`}
						class="icon-button"
						onclick={rotateColorMode}
						title={`Color mode: ${colorMode}`}
						type="button"
					>
						{#if colorMode === 'light'}
							<Sun size={16} strokeWidth={1.9} />
						{:else if colorMode === 'dark'}
							<Moon size={16} strokeWidth={1.9} />
						{:else}
							<Monitor size={16} strokeWidth={1.9} />
						{/if}
					</button>
				</div>
			</div>
		</header>

		<main class="workspace-main">
			{@render children()}
		</main>
	</div>
</div>

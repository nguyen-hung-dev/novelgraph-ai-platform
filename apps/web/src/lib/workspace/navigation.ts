export type MatchMode = 'exact' | 'prefix';

export type RouteHref =
	| '/projects'
	| '/settings'
	| '/settings/byok'
	| `/projects/${string}`
	| `/projects/${string}/import`
	| `/projects/${string}/reading`
	| `/projects/${string}/analysis`
	| `/projects/${string}/review`;

export type NavItem = {
	label: string;
	href: RouteHref;
	match: MatchMode;
};

export function isNavActive(currentPath: string, item: NavItem) {
	if (item.match === 'exact') {
		return currentPath === item.href;
	}

	return currentPath === item.href || currentPath.startsWith(`${item.href}/`);
}

export function buildProjectTabs(projectId: string): NavItem[] {
	const base = `/projects/${projectId}` as const;

	return [
		{ label: 'Overview', href: base, match: 'exact' },
		{ label: 'Import', href: `${base}/import`, match: 'prefix' },
		{ label: 'Reading', href: `${base}/reading`, match: 'prefix' },
		{ label: 'Analysis', href: `${base}/analysis`, match: 'prefix' },
		{ label: 'Review', href: `${base}/review`, match: 'prefix' }
	];
}

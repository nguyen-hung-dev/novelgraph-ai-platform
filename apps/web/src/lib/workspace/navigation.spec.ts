import { describe, expect, it } from 'vitest';
import { buildProjectTabs, isNavActive, type NavItem } from '$lib/workspace/navigation';

describe('workspace navigation', () => {
	it('matches exact routes only when requested', () => {
		const overview: NavItem = { label: 'Overview', href: '/projects/demo', match: 'exact' };

		expect(isNavActive('/projects/demo', overview)).toBe(true);
		expect(isNavActive('/projects/demo/reading', overview)).toBe(false);
	});

	it('matches nested routes for prefix sections', () => {
		const reading: NavItem = {
			label: 'Reading',
			href: '/projects/demo/reading',
			match: 'prefix'
		};

		expect(isNavActive('/projects/demo/reading', reading)).toBe(true);
		expect(isNavActive('/projects/demo/reading/ch-01', reading)).toBe(true);
	});

	it('builds stable project tabs', () => {
		expect(buildProjectTabs('ashen-archive')).toEqual([
			{ label: 'Overview', href: '/projects/ashen-archive', match: 'exact' },
			{ label: 'Import', href: '/projects/ashen-archive/import', match: 'prefix' },
			{ label: 'Reading', href: '/projects/ashen-archive/reading', match: 'prefix' },
			{ label: 'Analysis', href: '/projects/ashen-archive/analysis', match: 'prefix' },
			{ label: 'Review', href: '/projects/ashen-archive/review', match: 'prefix' }
		]);
	});
});

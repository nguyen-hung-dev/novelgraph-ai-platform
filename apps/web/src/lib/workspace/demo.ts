export type Tone = 'good' | 'warning' | 'danger' | 'teal' | 'neutral';

export type ProjectSummary = {
	id: string;
	name: string;
	summary: string;
	sourceLanguage: string;
	chapterCount: number;
	wordCount: number;
	stage: string;
	reviewQueue: number;
	activeModel: string;
	localModel: string;
	updatedAt: string;
	tags: string[];
};

export type ChapterSummary = {
	id: string;
	order: number;
	title: string;
	words: number;
	state: 'Ready' | 'Review' | 'Translated';
	note: string;
};

export type ReadingDocument = {
	chapterId: string;
	title: string;
	location: string;
	paragraphs: string[];
	entities: Array<{
		name: string;
		kind: string;
		notes: string[];
	}>;
	evidence: Array<{
		label: string;
		quote: string;
		confidence: string;
	}>;
};

export const projects: ProjectSummary[] = [
	{
		id: 'ashen-archive',
		name: 'Ashen Archive',
		summary:
			'Evidence-first extraction pilot with local chapter parsing, reviewable entities, and glossary-aware translation planning.',
		sourceLanguage: 'English',
		chapterCount: 124,
		wordCount: 518_000,
		stage: 'Local extraction dry run',
		reviewQueue: 18,
		activeModel: 'qwen3-32b',
		localModel: 'llama.cpp / qwen3',
		updatedAt: '2 minutes ago',
		tags: ['Mystery', 'Political fantasy', 'Translation-ready']
	},
	{
		id: 'river-maple',
		name: 'River of Maple Glass',
		summary:
			'Hosted web project stub for BYOK workflows, project sharing, and chapter-level translation review.',
		sourceLanguage: 'Chinese',
		chapterCount: 87,
		wordCount: 403_500,
		stage: 'Import preview locked',
		reviewQueue: 7,
		activeModel: 'DeepSeek-compatible',
		localModel: 'Remote only',
		updatedAt: 'Yesterday',
		tags: ['Cultivation', 'Shared review', 'Glossary draft']
	},
	{
		id: 'harbor-nine',
		name: 'Harbor Nine',
		summary:
			'Compact desktop dataset used to validate reading UX, chapter navigation, and issue triage without remote dependencies.',
		sourceLanguage: 'Vietnamese',
		chapterCount: 36,
		wordCount: 171_200,
		stage: 'Reading review',
		reviewQueue: 3,
		activeModel: 'qwen3-14b',
		localModel: 'llama.cpp / qwen3',
		updatedAt: '3 days ago',
		tags: ['Crime', 'Local only']
	}
];

export const releaseNotes = [
	{
		title: 'Workspace shell',
		copy: 'Desktop-style SvelteKit layout with sidebar, top toolbar, and project tabs.'
	},
	{
		title: 'Reading split pane',
		copy: 'Chapter list, source text, and evidence/review context now share one responsive workspace.'
	},
	{
		title: 'BYOK settings draft',
		copy: 'Provider form layout is in place before encrypted persistence and backend validation.'
	}
];

export const dashboardMetrics = [
	{
		label: 'Projects',
		value: '3',
		detail: '1 local-first pilot, 1 hosted web stub, 1 compact reading set',
		tone: 'accent'
	},
	{
		label: 'Queued reviews',
		value: '28',
		detail: 'Fact ambiguity, evidence gaps, translation terminology',
		tone: 'amber'
	},
	{
		label: 'Active runtimes',
		value: '2',
		detail: 'Rust API and local llama.cpp health surfaces prepared',
		tone: 'teal'
	},
	{
		label: 'Current mode',
		value: '0.7.0',
		detail:
			'Workspace retention, color-mode controls, and reading typography settings on top of live API wiring',
		tone: 'rose'
	}
] as const;

export const runtimeBadges = [
	{ label: 'API v0', tone: 'teal' as Tone },
	{ label: 'Schema v3', tone: 'good' as Tone },
	{ label: 'Local-first', tone: 'warning' as Tone }
];

export const chapters: ChapterSummary[] = [
	{
		id: 'ch-001',
		order: 1,
		title: 'Chapter 1 - Lanterns at South Gate',
		words: 4231,
		state: 'Translated',
		note: 'Stable heading detected from Markdown source.'
	},
	{
		id: 'ch-002',
		order: 2,
		title: 'Chapter 2 - The Quiet Ledger',
		words: 3984,
		state: 'Review',
		note: 'Two unresolved alias mentions around the archive clerk.'
	},
	{
		id: 'ch-003',
		order: 3,
		title: 'Chapter 3 - Smoke over the Canal',
		words: 4468,
		state: 'Ready',
		note: 'Good candidate for local draft extraction.'
	},
	{
		id: 'ch-004',
		order: 4,
		title: 'Chapter 4 - Salt Tax Hearing',
		words: 4816,
		state: 'Ready',
		note: 'Contains faction dispute and several timeline anchors.'
	}
];

export const chapterDocuments: Record<string, ReadingDocument> = {
	'ch-001': {
		chapterId: 'ch-001',
		title: 'Chapter 1 - Lanterns at South Gate',
		location: 'South Gate district, dusk',
		paragraphs: [
			'The gate lanterns had already been lit when Mira reached the south wall, but the record carts were still backed up along the stone ramp.',
			'She counted three clerks from the Ashen Archive and one excise guard with river mud on his boots, which meant the tax hearing had run late again.',
			'When the old porter refused to lift the crate, Mira touched the seal herself and felt fresh wax under soot, as if the ledger had been packed only an hour before.'
		],
		entities: [
			{
				name: 'Mira',
				kind: 'Character',
				notes: ['Courier', 'Likely point-of-view anchor for archive scenes']
			},
			{
				name: 'Ashen Archive',
				kind: 'Institution',
				notes: ['Appears in tax and records workflow']
			},
			{ name: 'South Gate', kind: 'Location', notes: ['Recurring urban checkpoint'] }
		],
		evidence: [
			{
				label: 'POV confidence',
				quote: 'Mira reached the south wall, but the record carts were still backed up',
				confidence: 'High'
			},
			{
				label: 'Institution mention',
				quote: 'three clerks from the Ashen Archive',
				confidence: 'High'
			}
		]
	},
	'ch-002': {
		chapterId: 'ch-002',
		title: 'Chapter 2 - The Quiet Ledger',
		location: 'Archive annex, late night',
		paragraphs: [
			'The annex sounded empty until the copying machine slowed, each crank clicking like a metronome against the rafters.',
			'Archivist Sen kept calling the sealed account book a quiet ledger, but the apprentice on night duty whispered that it belonged to the salt office.',
			'Mira did not challenge either of them. She only copied the page number and marked the broken ribbon in the margin.'
		],
		entities: [
			{
				name: 'Archivist Sen',
				kind: 'Character',
				notes: ['Alias uncertainty against later title references']
			},
			{ name: 'Salt Office', kind: 'Institution', notes: ['May connect to tax hearing thread'] }
		],
		evidence: [
			{
				label: 'Alias ambiguity',
				quote: 'Archivist Sen kept calling the sealed account book a quiet ledger',
				confidence: 'Medium'
			}
		]
	},
	'ch-003': {
		chapterId: 'ch-003',
		title: 'Chapter 3 - Smoke over the Canal',
		location: 'Canal quarter, before dawn',
		paragraphs: [
			'By dawn the canal quarter smelled of wet rope and cold smoke, and every door between the ferry stairs and the grain sheds had been barred from inside.',
			'Captain Ro left orders at the checkpoint for anyone carrying archive seals to report directly to the hearing chamber instead of the docks.',
			'Mira folded the order into her sleeve, then looked down at the canal and wondered why the water was carrying ash upstream.'
		],
		entities: [
			{
				name: 'Captain Ro',
				kind: 'Character',
				notes: ['Command authority tied to checkpoint logistics']
			},
			{ name: 'Canal quarter', kind: 'Location', notes: ['Potential hazard scene for timeline'] }
		],
		evidence: [
			{
				label: 'Anomaly flag',
				quote: 'the water was carrying ash upstream',
				confidence: 'Medium'
			}
		]
	},
	'ch-004': {
		chapterId: 'ch-004',
		title: 'Chapter 4 - Salt Tax Hearing',
		location: 'Public chamber, mid-morning',
		paragraphs: [
			'The public chamber had no windows facing the river, yet everyone inside seemed to know when the tide turned because the floorboards creaked at once.',
			'Deputy Halden held up the wax-sealed ledger and asked who had authorized its transfer from the archive vault before the hearing was called.',
			'Mira answered too slowly. By the time she spoke, the room had already decided she knew more about the missing crate than she was willing to admit.'
		],
		entities: [
			{
				name: 'Deputy Halden',
				kind: 'Character',
				notes: ['Factional authority; cross-check against prior mentions']
			},
			{
				name: 'Public chamber',
				kind: 'Location',
				notes: ['Likely node for timeline and conflict map']
			}
		],
		evidence: [
			{
				label: 'Conflict trigger',
				quote: 'who had authorized its transfer from the archive vault',
				confidence: 'High'
			},
			{
				label: 'Suspicion marker',
				quote: 'the room had already decided she knew more',
				confidence: 'High'
			}
		]
	}
};

export const importPreviewRows = [
	{ title: 'Chapter 1 - Lanterns at South Gate', detection: 'Markdown heading', status: 'Ready' },
	{ title: 'Chapter 2 - The Quiet Ledger', detection: 'Markdown heading', status: 'Ready' },
	{
		title: 'Chapter 3 - Smoke over the Canal',
		detection: 'Regex fallback',
		status: 'Needs review'
	},
	{ title: 'Chapter 4 - Salt Tax Hearing', detection: 'Regex fallback', status: 'Ready' }
];

export const splitWarnings = [
	'Chapter 3 shares the same heading depth as an appendix block. Confirm split boundary before persistence.',
	'Three scene separators were detected without explicit chapter labels after line 812.'
];

export const analysisRun = {
	id: 'analysis_local_20260426_01',
	status: 'running',
	stage: 'Schema-constrained extraction',
	completed: 38,
	total: 124,
	provider: 'llama.cpp',
	model: 'qwen3-32b-q4-k-m',
	queueDepth: 2
};

export const analysisEvents = [
	'Queued chapter bundle 05-08 for local draft extraction.',
	'Chapter 3 flagged one uncertain institution alias.',
	'Translation glossary sync skipped because style profile is still draft.',
	'Observation persistence remains disabled for local draft runs.'
];

export const failedChapters = [
	{ title: 'Chapter 17 - Drowned Prayer', reason: 'JSON schema mismatch after retry limit' },
	{ title: 'Chapter 41 - Red Corridor', reason: 'Evidence span index drifted after source edit' }
];

export const reviewQueue = [
	{
		id: 'review-001',
		title: 'Archivist Sen alias conflict',
		chapter: 'Chapter 2',
		severity: 'warning' as Tone,
		summary: 'The same person may appear later as Sen-ji in translation notes.',
		evidence: 'Archivist Sen kept calling the sealed account book a quiet ledger.',
		recommendation: 'Lock canonical name only after glossary review.'
	},
	{
		id: 'review-002',
		title: 'Missing transfer authority',
		chapter: 'Chapter 4',
		severity: 'danger' as Tone,
		summary: 'Authorization source for the vault transfer is implied but not explicitly stated.',
		evidence: 'who had authorized its transfer from the archive vault',
		recommendation: 'Keep as unresolved review item instead of asserting a faction.'
	},
	{
		id: 'review-003',
		title: 'Canal anomaly',
		chapter: 'Chapter 3',
		severity: 'teal' as Tone,
		summary: 'Possible worldbuilding clue rather than plot fact.',
		evidence: 'the water was carrying ash upstream',
		recommendation: 'Route to world-state extraction, not character graph.'
	}
];

export const providerProfiles = [
	{
		name: 'Local llama.cpp',
		mode: 'Session only',
		baseUrl: 'http://127.0.0.1:8080',
		model: 'qwen3-32b-q4-k-m',
		status: 'Connected'
	},
	{
		name: 'OpenAI-compatible',
		mode: 'BYOK pending',
		baseUrl: 'https://api.example.com/v1',
		model: 'gpt-4.1-mini',
		status: 'Not validated'
	}
];

export function findProjectById(projectId: string) {
	return projects.find((project) => project.id === projectId);
}

<script lang="ts">
	import { onDestroy, onMount } from 'svelte';
	import type { ECharts, EChartsCoreOption } from 'echarts/core';

	export let title = '';
	export let option: EChartsCoreOption;
	export let height = 280;
	let node: HTMLDivElement;
	let chart: ECharts | null = null;
	let mounted = false;
	let loadError = '';
	let removeResize: (() => void) | null = null;
	let themeObserver: MutationObserver | null = null;

	type EChartsCore = typeof import('echarts/core');
	type ThemeColors = {
		text: string;
		textSecondary: string;
		textSoft: string;
		textMuted: string;
		textDim: string;
		textFaint: string;
		bg: string;
		surface: string;
		border: string;
		borderStrong: string;
		borderBright: string;
		series: string[];
	};

	let chartModules: Promise<EChartsCore> | null = null;

	function loadECharts(): Promise<EChartsCore> {
		chartModules ??= Promise.all([
			import('echarts/core'),
			import('echarts/charts'),
			import('echarts/components'),
			import('echarts/renderers')
		]).then(([echarts, charts, components, renderers]) => {
			echarts.use([
				charts.BarChart,
				charts.HeatmapChart,
				charts.LineChart,
				charts.PieChart,
				charts.ScatterChart,
				charts.TreemapChart,
				components.GridComponent,
				components.LegendComponent,
				components.TooltipComponent,
				components.VisualMapComponent,
				renderers.CanvasRenderer
			]);
			return echarts;
		});
		return chartModules;
	}

	$: if (chart && option) setChartOption();

	function baseOption(input: EChartsCoreOption): EChartsCoreOption {
		const colors = themeColors();
		const tooltip = typeof input.tooltip === 'object' && !Array.isArray(input.tooltip) ? input.tooltip : {};
		const textStyle = typeof input.textStyle === 'object' && !Array.isArray(input.textStyle) ? input.textStyle : {};
		const themed = themeChartColors({
			backgroundColor: 'transparent',
			color: colors.series,
			textStyle: { color: colors.textSecondary, fontFamily: 'Sora Local, system-ui', ...textStyle },
			tooltip: {
				backgroundColor: colors.text,
				borderColor: colors.text,
				textStyle: { color: colors.bg },
				...tooltip
			},
			grid: { left: 40, right: 18, top: 22, bottom: 34 },
			...input
		}, colors) as EChartsCoreOption;

		return {
			...themed,
			tooltip: {
				...(themed.tooltip as Record<string, unknown>),
				...(themeChartColors(tooltip, colors) as Record<string, unknown>)
			}
		};
	}

	function setChartOption() {
		chart?.setOption(baseOption(option), true);
	}

	function themeColors(): ThemeColors {
		const styles = getComputedStyle(document.documentElement);
		const css = (name: string) => styles.getPropertyValue(name).trim();
		return {
			text: css('--color-text'),
			textSecondary: css('--color-text-secondary'),
			textSoft: css('--color-text-soft'),
			textMuted: css('--color-text-muted'),
			textDim: css('--color-text-dim'),
			textFaint: css('--color-text-faint'),
			bg: css('--color-bg'),
			surface: css('--color-surface'),
			border: css('--color-border'),
			borderStrong: css('--color-border-strong'),
			borderBright: css('--color-border-bright'),
			series: [
				css('--chart-series-1'),
				css('--chart-series-2'),
				css('--chart-series-3'),
				css('--chart-series-4'),
				css('--chart-series-5'),
				css('--chart-series-6'),
				css('--chart-series-7'),
				css('--chart-series-8')
			]
		};
	}

	function themeChartColors(value: unknown, colors: ThemeColors): unknown {
		if (typeof value === 'string') return themedColor(value, colors);
		if (Array.isArray(value)) return value.map((item) => themeChartColors(item, colors));
		if (!value || typeof value !== 'object') return value;
		return Object.fromEntries(Object.entries(value).map(([key, item]) => [key, themeChartColors(item, colors)]));
	}

	function themedColor(value: string, colors: ThemeColors) {
		const normalized = value.toLowerCase();
		const replacements: Record<string, string> = {
			'#f5f5f5': colors.series[0],
			'#e5e5e5': colors.series[0],
			'#d4d4d4': colors.series[1],
			'#a3a3a3': colors.textSoft,
			'#8a8a8a': colors.textMuted,
			'#737373': colors.textDim,
			'#525252': colors.textFaint,
			'#404040': colors.borderBright,
			'#333': colors.borderStrong,
			'#262626': colors.border,
			'#050505': colors.bg
		};
		return replacements[normalized] ?? value;
	}

	onMount(() => {
		mounted = true;
		void loadECharts()
			.then((echarts) => {
				if (!mounted) return;

				chart = echarts.init(node, undefined, { renderer: 'canvas' });
				setChartOption();
				const resize = () => chart?.resize();
				window.addEventListener('resize', resize);
				removeResize = () => window.removeEventListener('resize', resize);
				themeObserver = new MutationObserver(setChartOption);
				themeObserver.observe(document.documentElement, { attributes: true, attributeFilter: ['data-theme', 'data-palette'] });
			})
			.catch((error) => {
				loadError = error instanceof Error ? error.message : 'Unable to load chart renderer';
			});
	});

	onDestroy(() => {
		mounted = false;
		removeResize?.();
		themeObserver?.disconnect();
		chart?.dispose();
	});
</script>

<section class="chart-card">
	<header>{title}</header>
	{#if loadError}
		<div class="chart-error" style={`height:${height}px`}>{loadError}</div>
	{:else}
		<div bind:this={node} style={`height:${height}px`}></div>
	{/if}
</section>

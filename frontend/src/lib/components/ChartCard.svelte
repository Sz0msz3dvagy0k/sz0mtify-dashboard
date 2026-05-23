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

	type EChartsCore = typeof import('echarts/core');

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

	$: if (chart && option) chart.setOption(baseOption(option), true);

	function baseOption(input: EChartsCoreOption): EChartsCoreOption {
		return {
			backgroundColor: 'transparent',
			textStyle: { color: '#d4d4d4', fontFamily: 'Sora Local, system-ui' },
			tooltip: {
				backgroundColor: '#f5f5f5',
				borderColor: '#f5f5f5',
				textStyle: { color: '#050505' }
			},
			grid: { left: 40, right: 18, top: 22, bottom: 34 },
			...input
		};
	}

	onMount(() => {
		mounted = true;
		void loadECharts()
			.then((echarts) => {
				if (!mounted) return;

				chart = echarts.init(node, undefined, { renderer: 'canvas' });
				chart.setOption(baseOption(option), true);
				const resize = () => chart?.resize();
				window.addEventListener('resize', resize);
				removeResize = () => window.removeEventListener('resize', resize);
			})
			.catch((error) => {
				loadError = error instanceof Error ? error.message : 'Unable to load chart renderer';
			});
	});

	onDestroy(() => {
		mounted = false;
		removeResize?.();
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

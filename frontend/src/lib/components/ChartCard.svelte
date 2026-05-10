<script lang="ts">
	import { onDestroy, onMount } from 'svelte';
	import * as echarts from 'echarts/core';
	import { BarChart, HeatmapChart, LineChart, PieChart, ScatterChart, TreemapChart } from 'echarts/charts';
	import {
		GridComponent,
		LegendComponent,
		TooltipComponent,
		VisualMapComponent
	} from 'echarts/components';
	import { CanvasRenderer } from 'echarts/renderers';
	import type { EChartsCoreOption } from 'echarts/core';

	echarts.use([
		BarChart,
		HeatmapChart,
		LineChart,
		PieChart,
		ScatterChart,
		TreemapChart,
		GridComponent,
		LegendComponent,
		TooltipComponent,
		VisualMapComponent,
		CanvasRenderer
	]);

	export let title = '';
	export let option: EChartsCoreOption;
	export let height = 280;
	let node: HTMLDivElement;
	let chart: echarts.ECharts | null = null;

	$: if (chart && option) chart.setOption(baseOption(option), true);

	function baseOption(input: EChartsCoreOption): EChartsCoreOption {
		return {
			backgroundColor: 'transparent',
			textStyle: { color: '#d4d4d4', fontFamily: 'Inter, system-ui' },
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
		chart = echarts.init(node, undefined, { renderer: 'canvas' });
		chart.setOption(baseOption(option), true);
		const resize = () => chart?.resize();
		window.addEventListener('resize', resize);
		return () => window.removeEventListener('resize', resize);
	});

	onDestroy(() => chart?.dispose());
</script>

<section class="chart-card">
	<header>{title}</header>
	<div bind:this={node} style={`height:${height}px`}></div>
</section>

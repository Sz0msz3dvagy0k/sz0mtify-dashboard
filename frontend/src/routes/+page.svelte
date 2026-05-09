<script lang="ts"> const pages=['overview','audio-quality','albums','artists','genres','timeline','storage','metadata-health','listening','discovery','search','settings']; </script>
<div class="container"><h1>Music Dashboard</h1><div class="grid">{#each pages as p}<a class="card" href={'/'+p}>{p}</a>{/each}</div></div>

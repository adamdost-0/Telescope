<script lang="ts">
  let { data = [], width = 120, height = 32, color = '#58a6ff' }: {
    data: number[];
    width?: number;
    height?: number;
    color?: string;
  } = $props();

  let points = $derived.by(() => {
    if (data.length < 2) return '';
    const max = Math.max(...data, 1);
    const min = Math.min(...data, 0);
    const range = max - min || 1;
    return data.map((v, i) => {
      const x = (i / (data.length - 1)) * width;
      const y = height - ((v - min) / range) * (height - 4) - 2;
      return `${x},${y}`;
    }).join(' ');
  });
</script>

<svg {width} {height} class="sparkline" aria-label="Usage trend">
  {#if points}
    <polyline fill="none" stroke={color} stroke-width="1.5" points={points} />
  {:else}
    <text x="50%" y="50%" text-anchor="middle" fill="#484f58" font-size="10">—</text>
  {/if}
</svg>

<style>
  .sparkline { display: inline-block; vertical-align: middle; }
</style>

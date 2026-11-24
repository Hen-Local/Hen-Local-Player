<script lang="ts">
  import { player as p } from './stores.svelte'

  const controls = [
    { key: 'subtitleScale', label: 'scale', min: 0.6, max: 2, step: 0.1 },
    { key: 'subtitleBottomOffset', label: 'offset', min: 0, max: 0.4, step: 0.01 },
    { key: 'subtitleBackground', label: 'background', min: 0, max: 0.8, step: 0.05 },
  ] as const
</script>

<div class="subtitle-appearance">
  {#each controls as control}
    {@const percent = Math.round(p.settings[control.key] * 100) + '%'}
    <label class="slider-setting">
      <span>{p.t(control.label)}<b>{percent}</b></span>
      <input
        type="range"
        min={control.min}
        max={control.max}
        step={control.step}
        value={p.settings[control.key]}
        aria-label={p.t(control.label)}
        aria-valuetext={percent}
        oninput={(e) => { p.settings[control.key] = e.currentTarget.valueAsNumber }}
        onchange={(e) => {
          const value = e.currentTarget.valueAsNumber
          void p.act(() => p.update({ [control.key]: value }))
        }}
      />
    </label>
  {/each}
</div>

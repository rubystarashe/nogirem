<script>
  let {
    blocks = [],
    onlink = () => {},
  } = $props()

  function openLink(event, block) {
    event.preventDefault()
    onlink(block)
  }
</script>

{#each blocks as block}
  {#if block.type === "heading"}
    {#if block.level === 1}
      <h1>{block.text}</h1>
    {:else if block.level === 2}
      <h2>{block.text}</h2>
    {:else}
      <h3>{block.text}</h3>
    {/if}
  {:else if block.type === "list"}
    <ul>
      {#each block.items as item}
        <li>{item}</li>
      {/each}
    </ul>
  {:else if block.type === "image"}
    <img
      class="markdown-image"
      src={block.src}
      alt={block.alt}
      draggable="false"
    />
  {:else if block.type === "link"}
    <a
      href={block.href}
      class="markdown-link"
      onclick={event => openLink(event, block)}
    >
      {block.text}
    </a>
  {:else}
    <p>{block.text}</p>
  {/if}
{/each}

<script lang="ts">
  import UiButton from "$lib/components/ui/Button.svelte";
  import Field from "$lib/components/ui/Field.svelte";
  import TextInput from "$lib/components/ui/TextInput.svelte";

  let {
    editing = false,
    saving = false,
    label = $bindable(""),
    loginName = $bindable(""),
    password = $bindable(""),
    onReset,
    onSave,
  }: {
    editing?: boolean;
    saving?: boolean;
    label?: string;
    loginName?: string;
    password?: string;
    onReset: () => void;
    onSave: (event: SubmitEvent) => void;
  } = $props();
</script>

<section class="account-editor" aria-label="Account editor">
  <div class="panel-title">
    <div>
      <h2>{editing ? "Account details" : "Add account"}</h2>
      <p>{editing ? "Editing selected source" : "New DLsite source"}</p>
    </div>
    <UiButton variant="secondary" size="small" disabled={saving} onclick={onReset}>New</UiButton>
  </div>

  <form onsubmit={onSave}>
    <div class="account-form-grid">
      <Field id="account-label" label="Label">
        <TextInput id="account-label" autocomplete="off" disabled={saving} bind:value={label} />
      </Field>
      <Field id="account-login" label="Login">
        <TextInput
          id="account-login"
          autocomplete="username"
          spellcheck={false}
          disabled={saving}
          bind:value={loginName}
        />
      </Field>
      <Field id="account-password" label="Password">
        <TextInput
          id="account-password"
          type="password"
          autocomplete="current-password"
          disabled={saving}
          bind:value={password}
        />
      </Field>
    </div>
    <div class="form-actions">
      <span>{editing ? "Update source" : "Create source"}</span>
      <UiButton type="submit" disabled={saving}>{editing ? "Save" : "Add"}</UiButton>
    </div>
  </form>
</section>

<style>
  .account-editor {
    position: sticky;
    top: 28px;
    display: grid;
    gap: 14px;
    padding: 18px;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--panel);
    box-shadow: 0 16px 40px rgb(0 0 0 / 18%);
  }

  .panel-title,
  .form-actions {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
  }

  .panel-title {
    align-items: flex-start;
  }

  .panel-title > div {
    min-width: 0;
  }

  h2 {
    margin: 0;
    color: var(--text-strong);
    font-size: 17px;
    font-weight: 700;
  }

  p {
    margin: 4px 0 0;
    color: var(--muted);
    font-size: 12px;
    line-height: 1.35;
  }

  form,
  .account-form-grid {
    display: grid;
    gap: 14px;
  }

  .form-actions span {
    color: var(--muted);
    font-size: 12px;
  }

  @media (max-width: 980px) {
    .account-editor {
      position: static;
    }
  }

  @media (max-width: 720px) {
    .panel-title,
    .form-actions {
      align-items: stretch;
      flex-direction: column;
    }
  }
</style>

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

<form onsubmit={onSave} class="dm:flex dm:flex-col dm:gap-4">
  <Field id="account-label" label="Account name"
    ><TextInput
      id="account-label"
      autocomplete="off"
      disabled={saving}
      bind:value={label}
    /></Field
  >
  <Field id="account-login" label="Login"
    ><TextInput
      id="account-login"
      autocomplete="username"
      spellcheck={false}
      disabled={saving}
      bind:value={loginName}
    /></Field
  >
  <Field id="account-password" label="Password"
    ><TextInput
      id="account-password"
      type="password"
      autocomplete="current-password"
      placeholder={editing ? "Leave blank to keep saved password" : ""}
      disabled={saving}
      bind:value={password}
    /></Field
  >
  <div class="dm:flex dm:justify-end">
    <UiButton type="submit" disabled={saving}
      >{saving ? "Saving…" : "Save account"}</UiButton
    >
  </div>
</form>

# Debug Mode

Debug mode lets you replace a running container's command with `sleep infinity` so you can exec into it for investigation without the application process interfering.

Select a **Pod** and press `Ctrl+Alt+D` to toggle debug mode. KubeTile will:

1. Resolve the Pod's owner Deployment via its ReplicaSet.
2. Save the original `command` and `args` as annotations on the Deployment.
3. Patch the Deployment to run `sleep infinity` — all pods in the Deployment restart.
4. Press the same key again to restore the original command and exit debug mode.

## Root Debug Mode

Press `F5` on a Pod to toggle **root debug mode**. This does everything debug mode does, and additionally sets:

```yaml
securityContext:
  runAsUser: 0
```

This is useful when you need root access inside the container (e.g., to inspect system files or install debugging tools). The original `securityContext` is preserved and restored on exit.

> **Safety:** if the Deployment is already in one debug mode when you activate the other, the original application command is never overwritten — it is reused from the existing saved annotation.

---

## Keybindings

| Key | Action |
|-----|--------|
| `Ctrl+Alt+D` | Toggle debug mode |
| `F5` | Toggle root debug mode |

---

See also: [Resource List](resource-list.md) · [Keybindings reference](../keybindings.md)

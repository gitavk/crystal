  Here's what's genuinely repeated vs what just looks similar:

  Worth extracting (identical, repeated 4×):
  fetch deployment API + deploy + first container name
  This 10-line block is literally copy-pasted across all four enter/exit_* functions.

  Not worth extracting:
  - The patch JSON bodies — structurally different enough that unifying them behind a bool/enum would add more conditional noise than it removes
  - is_in_* — each is 3 lines with different logic; a shared helper would be longer than both combined
  - The annotation reuse guard — it's only in 2 functions and diverges slightly (enter_root_debug_mode also reads security_context)

  The actual DRY win is one private helper:
  async fn fetch_deploy_first_container(name, ns) -> Result<(Api<Deployment>, Deployment, String)>
  returning the container name as String (not a reference, to stay borrowck-safe).

  Net effect: ~30 lines removed, the 4 public functions become clearly focused on just building their specific patch. No added complexity.

resource "hcloud_ssh_key" "default" {
  name       = "hetzner_zk_key"
  public_key = file("~/.ssh/tf_hezner.pub")
}
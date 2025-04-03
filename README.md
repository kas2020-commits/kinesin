<p align="center">
  <img src="https://github.com/user-attachments/assets/0341783e-cd11-4535-a075-e3775f779184" alt="kinesin logo" style="width: 30%; height: auto;">
</p>

**Kinesin** is a lightweight init system and process supervisor for containerized environments, written in Rust! It provides reliable startup, child reaping and management of child processes, with built-in support for stream redirection, external logging, post-death cleanup jobs and more.

Named after the molecular motor *kinesin*, which transports cellular cargo along microtubules, this project carries your subprocesses with precision, structure, and speed.

---

## ✨ Features

- 🚀 **Initialize Multiple Processes** – Acts as PID 1 in containers, managing lifecycles cleanly.
- 🧵 **Stream Redirection** – Flexible redirection of `stdin`, `stdout` and `stderr`, using a bus with multiple consumers on the end.
- 🚰 **Buffered Streaming** - Configure per-bus buffering limits to prevent writing to sensitive consumers too often.
- 🦀 **Built with Rust** – Fast, safe, and modern systems-level development.

---

## 📦 Use Cases

- Bundle simple daemons into your container without needing pods or networking configs
- Inject post-death sequences/scripts to alert operators when a container dies.
- Redirect logs from stdout to Grafana+Loki with 0 code changes to your app.
- Reliable PID 1 behavior with child reaping.
- Periodic health checks with user-defined concepts of health.
- Use the same tool for both development and production with no changes.

---

## Q & A

### Q: Why Another Supervision Suite?

There's a [great article](https://skarnet.org/software/s6/why.html) from the s6 project of the same title which covers a lot of good points about process supervision design. Unfortunately though, many supervisior systems predate the popularization of linux's cgroups and namespacing, meaning they're often designed to run as Gods. For example, [Systemd tries mounting a tmpfs on `/run`](https://youtu.be/93VPog3EKbs?si=P53gUlzX8yomcD1O&t=1407) because that's what it believes in. This wouldn't a problem on normal systems but it's unecessary inside a container, forcing the need for `--privileged` mode.

That's not the only problem though, and you'll run into many more bits in these longer standing init systems that are either unecessary in containers or outright not supported. Kinesin isn't designed to be PID 1 on a desktop or shiped with a distro. Rather, it's designed to run thin, lightweight containers quietly and with as little disruption as possible while providing the features that it does.

### Q: Why not Sidecars or DaemonSets?

You might wonder why not just use the namespacing tools and add it to your Kubernetes configuration? Well, there's many reasons why that's not ideal for you:

- You don't run Kubernetes in prod (gasp!)
- You're not the operator and can't/won't elevate a support ticket to get it done
- You need a system that's reproducible in your development environment
- You or you team are not the operators/owners - you're soley responsible for producing an OCI image but still need to bundle other tools inside

I could keep making this list longer, but there's a fundamental difference here underpinning all these points: sidecars are an *inter*-container tool, not an *intra*-container tool.

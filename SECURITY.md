# Security Policy

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| 0.1.x   | :white_check_mark: |

## Reporting a Vulnerability

We take security vulnerabilities seriously. If you discover a security issue, please report it responsibly.

### How to Report

**Please DO NOT open a public GitHub issue for security vulnerabilities.**

Instead, report vulnerabilities via email:

**Email**: hephaex@gmail.com

**Subject**: `[SECURITY] k8s_vGateway - Brief Description`

### What to Include

Please provide the following information:

1. **Description**: Clear description of the vulnerability
2. **Impact**: Potential impact and severity assessment
3. **Steps to Reproduce**: Detailed steps to reproduce the issue
4. **Affected Versions**: Which versions are affected
5. **Possible Fix**: If you have suggestions for remediation

### Example Report

```
Subject: [SECURITY] k8s_vGateway - Command Injection in Deploy Module

Description:
A command injection vulnerability exists in the deploy module
when handling user-provided gateway names.

Impact:
An attacker could execute arbitrary commands on the host system.

Steps to Reproduce:
1. Run: gateway-poc deploy install "nginx; rm -rf /"
2. Observe command execution

Affected Versions:
- v0.1.0

Suggested Fix:
Sanitize user input before passing to shell commands.
```

## Response Timeline

| Action | Timeline |
|--------|----------|
| Initial Response | Within 48 hours |
| Vulnerability Assessment | Within 7 days |
| Fix Development | Depends on severity |
| Security Advisory | Upon fix release |

### Severity Levels

| Severity | Description | Response Time |
|----------|-------------|---------------|
| Critical | Remote code execution, data breach | 24-48 hours |
| High | Privilege escalation, significant data exposure | 7 days |
| Medium | Limited impact vulnerabilities | 14 days |
| Low | Minor issues, hardening suggestions | 30 days |

## Security Best Practices

When using k8s_vGateway:

### Configuration Security

- Never commit configuration files with secrets
- Use environment variables for sensitive data
- Restrict file permissions on config files

```bash
chmod 600 gateway-poc.yaml
```

### Kubernetes Security

- Use RBAC with least privilege principle
- Run in dedicated namespaces
- Enable network policies
- Use Pod Security Standards

### Credential Management

- Rotate credentials regularly
- Use Kubernetes secrets for sensitive data
- Avoid hardcoding credentials

## Security Features

### Built-in Protections

- Input validation on all user inputs
- Secure default configurations
- No credential storage in logs
- TLS support for all communications

### Recommended Setup

```yaml
# Example secure configuration
app:
  log_level: info  # Avoid debug in production

kubernetes:
  namespace: gateway-system
  rbac_enabled: true
```

## Disclosure Policy

1. **Private Disclosure**: Reporter contacts maintainers privately
2. **Assessment**: Maintainers assess and confirm vulnerability
3. **Fix Development**: Patch is developed and tested
4. **Coordinated Release**: Fix released with security advisory
5. **Public Disclosure**: Details published after users can update

## Recognition

We appreciate responsible disclosure and will:

- Acknowledge reporters in security advisories (unless anonymity requested)
- Work with reporters on fix timeline
- Provide updates on remediation progress

## Contact

- **Security Email**: hephaex@gmail.com
- **PGP Key**: Available upon request

## References

- [Rust Security Guidelines](https://rustsec.org/)
- [Kubernetes Security Best Practices](https://kubernetes.io/docs/concepts/security/)
- [OWASP Security Guidelines](https://owasp.org/)

# L696 / L590 canonical Omega installer delivery

Source aien-dev/aienos.com branch feat/omega-installer-with-site-prerequisites,
HEAD541682e79d1c46db7d0dad7d7dd186e373949ef2,
tree7e3499e2acc3747fad989b705476562ef77ad1d4,
public main base89683470c430829cef2bcc871dc90657fedac3ab.

This carries existing qualified L633 source1a76ee4 unchanged, including its
L540Pages repair, L545dashboard and CUDA discovery content. The only addition
over that carrier is48-line public/omega/install.sh. It supersedes older
installer-only82efe5a as the website integration candidate. Use ONE combined
website candidate or dependency-safe sequence; do not merge competing carriers.

Parent ran exact mandatory Mac recipe on this clean source:
140Rust testsPASS, releasebuildPASS,8collector testsPASS. Sourcecleanliness and
head/tree checked before/after. Generatedinstaller byte-identical topublicasset,
SHA256e4d6707b4023f8db9ffbd15fe603e8c67523603ba7d1c13640d6f75256cc1a36.
Immutablepublicsource payload eaf37ff32d17166de403372069b52aeb98b9523ce154b8f065ae9e4c161c8480
already passed actual publiccurl project/aliases/uninstall/refusal gates onPi5.

Receipt15ad60c0393f6a00d86f71a940fd1dcfc0bcd51ec0f45d5227bf25fbc1159089
is canonicalJSON and signed by the existing Mac Ed25519 qualification identity.
The existing private key stayed onMac, nosecretbytes were read into chat or
transferred. Its derived public identity matched the deployed Renda trust root;
no key rotation or deployment configuration change occurred. Mandatory commands
were not changed/skipped to accommodate the older installer-only branch's
missing collector; actual existing prerequisite source was reused instead.

Public/source installation intent after normal Renda merge and Pages deploy:
curl -fsSL https://www.aienos.com/omega/install.sh | sh
Support: experimentalLinuxARM64only. Shell hostglue and checksum-not-signedrelease
trustboundary remain disclosed. NoMacfulltoolchain orGPUinstallclaim.

Renda retains full-diff review, exactremotehead, branchrequirements, merge and
Pagesdeployment authority. Signed qualification is NOT merge/deployment approval.
L696/L590 stayOPEN until literal canonicalURL freshPi5 replay passes. Existing
L539/L633 owners are notified; no newpublisher, agentrestart or servicechange.

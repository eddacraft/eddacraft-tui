# DOCRB-008 public IA and curated diagram evidence

**Date:** 2026-08-21
**Item:** DOCRB-008
**Pinned starting receipt:** `7809a06801c583a740dda4abc18828d3a4970fdb`
**Result:** Executor evidence GREEN; independent verification and Council remain lifecycle gates.

## Scope and authority

The implementation is limited to four label/grouping-only production sidebars,
two owning-page references, exactly two Draw.io/SVG pairs, the one public-diagram
inventory note, this item's action/module/index/evidence records, and the directly
approved two-file exporter prerequisite, plus the binding file-level freshness
closeouts in `docs/guides/architecture-diagrams.md`,
`docs/guides/documentation-governance.md`, `docs/guides/README.md`,
`docs/architecture/README.md`, `docs/governance/tags-catalogue.md`,
`docs/guides/adapters/README.md`, `docs/reviews/README.md`, and `docs/README.md`.
The final approved implementation scope is 25 paths. No
public content was moved or rewritten;
`docs/public/aps/workflow.md`, the APS sidebar, generated references,
dependencies, lockfiles, PR #4050, CIB, DOCRB-009, and DOCRB-010 are untouched.

The operator directly approved the prerequisite on 2026-08-21 after authentic
Draw.io Desktop 31.1.8 output proved that the shipped exporter preserved the
official three-line prolog while the shipped checker rejected any processing
instruction or doctype. The repair removes only that exact anchored LF/CRLF
prolog before annotation and hashing. `hasUnsafeSvg` remains unchanged and
fail-closed.

The installed `aps start DOCRB-008` could not parse the repository's valid
`Merged <date> via PR #N` extension and reported the three Merged dependencies
as unknown/unmet. Their source rows and receipts were verified at the pinned
base, and the operator authorised a manual DOCRB-008-only transition through
Ready to In Progress. The current-item summary and index now reflect that state
while the stored 8/11 aggregate and every other item remain unchanged.

## Initial Council finding closure

The exact-head Council review initially blocked on three deduplicated findings:
stale Draft wording in the module summary and plan index; transparent diagram
canvases whose title and subtitle colours lacked contrast on the default dark
page; and freshness metadata pinned to commit IDs orphaned by unpublished
amends. The operator authorised FIX ALL within a final 25-path ceiling.

The repair reconciles only DOCRB-008's current lifecycle wording, adds one
opaque light canvas behind the existing content in each of the two approved
diagram sources, and replaces the orphaned branch IDs with durable links to
this action plan and evidence record. Nodes, labels, arrows, content authority,
navigation, stored progress, and all other items remain unchanged. Independent
verification and one scoped exact-head Council re-review remain required before
publication.

## TDD evidence

- Sidebar RED: the four production sidebars did not explicitly expose all
  relevant Diátaxis types. GREEN: labels now expose tutorial, how-to, reference,
  and explanation placement while a fresh pinned-base structural comparison
  proves the document ID/order sequences are unchanged: anvil 47, beta 1,
  kindling 16, edda-stack 7.
- Prerequisite RED: the exporter-wrapper fixture emitted the exact official
  Draw.io prolog and the final SVG still contained the XML declaration/doctype.
- Prerequisite GREEN: `node --test scripts/docs/check-public-diagrams.test.mjs`
  passes 80/80. The focused behaviours cover exact official-prolog success,
  altered PUBLIC URL, entity-bearing doctype, SYSTEM doctype, another processing
  instruction, deterministic hash tampering, and the existing active-XML,
  namespace, URL, symlink, version, and path fail-closed cases.
- Live corpus: `pnpm docs:public:diagrams` reports 4 files, 0 errors, and
  0 warnings.

## Diagram source, export, and accessibility trace

| Journey | Owning page | Canonical source SHA-256 | Raw official SVG SHA-256 | Annotated SVG SHA-256 |
| --- | --- | --- | --- | --- |
| anvil detect-fix-verify | `docs/public/anvil/first-gate.md` | `85e59340e1b7a0c148fa45c6b0083b5d9ee9d7978a38b40525f15a81e7dccf25` | `969b1ba0025aaa5d30c67d5f02516995b40211cfa83ed4cccd0b376527548f5b` | `5be1cdf6db55f616879ab5d5a95a3c8a02480d952151c467cf3e0680ff8161c9` |
| APS work-item lifecycle | `docs/public/aps/getting-started.md` | `fa5f87988badbf5c52b8222bf208b58f719c09a420bfd21f0e02e8a768e97a36` | `8c5409f3bfb03d59f4c8fcab12b1246717deb6f93b1cfed5b6d95bcc9863a42e` | `ccbb59b38bcb337daaa1c8bc327c2b0b50c05c53e6ac852950eb4c20a33d1d49` |

For both pairs, the official embedded `mxfile` is byte-identical to the sibling
canonical `.drawio` source after the single authorised canonicalisation pass.
That pass changed only Draw.io's non-semantic `mxGraphModel dy` viewport values;
all cells, text, styles, and content geometry remained identical.
The checker validates single-page source, embedded-source parity, authentic
version provenance, deterministic source/export hashes, lower-kebab naming,
mounted ownership, meaningful alt text, and unsafe-SVG rejection.

Manual inspection used temporary rsvg rasterisations of the committed SVG bytes
and installed headless Chromium against the two built owning pages. Both routes
returned 200 and loaded the complete images at their natural dimensions:
966×338 for anvil and 1096×350 for APS. Explicit light and dark browser contexts
selected the matching mounted page theme; all four captures show opaque canvases,
unclipped labels and arrows, and the intended sequence and loop/continuation
path. The source nodes match the owning prose. The APS page remains mounted and
links onward to the existing workflow authority.

The official export uses an opaque `background` cell across the full canvas.
Draw.io emits light/dark colour pairs for that cell and its text. Measured WCAG
contrast is 12.67:1 for the 24 px bold title and 7.56:1 for the 14 px subtitle in
light mode, and 10.56:1 and 7.61:1 respectively in dark mode. These exceed the
Council thresholds of 3:1 and 4.5:1.

## Authentic export environment

- Release authority: official jgraph Draw.io Desktop release `v31.1.8`.
- Official amd64 Debian asset SHA-256:
  `f4c49ed84422ea4afd95818f53c54bc666e57b33bd036d468c7096619b47ffd9`.
- Official Ubuntu image:
  `ubuntu@sha256:33ceb71981b602c1a7443a53469e4dba065f7503eab3078a2d7a57a2ab987517`.
- Draw.io version gate: stdout exactly `31.1.8\n`; stderr 0 bytes.
- Package index and download used the same exact image as UID/GID 1000:1000
  with dropped capabilities, `no-new-privileges`, bridge networking rather
  than host networking, one exchange bind, and no installation. All 157
  redownloaded package bytes match the recorded version/digest rows below.
- Offline export container identity/configuration:
  `User=1000:1000`, `CapDrop=[ALL]`,
  `SecurityOpt=[no-new-privileges:true]`, `NetworkMode=none`,
  `PidsLimit=256`, `ShmSize=268435456`; no repository mount; one
  read-write dedicated exchange bind, one read-only bind from the same exchange
  for Xvfb's packaged `xkbcomp` absolute path, and an isolated root-owned X11
  socket tmpfs. Xvfb used `-nolisten tcp`.
- The official packages were downloaded from the host-configured Ubuntu
  repositories, verified locally, and extracted only into the disposable
  environment. Nothing was installed on the host and no repository dependency
  or lockfile changed.
- Draw.io emitted one raw environment-only
  `g_settings_schema_source_lookup: assertion 'source != NULL' failed`
  warning per approved export. The operator accepted it as non-fidelity after a
  clean fixture; rooting Fontconfig at the verified extracted packages removed
  the earlier font warning. Chromium/rsvg inspection, embedded-source parity,
  stable hashes, and the repository checker all remained GREEN.
- The repository exporter was invoked with explicit `--root` successfully.
  No pending CI-log note exists at closeout, so this report makes no tooling
  follow-up claim.

<details>
<summary>Exact Ubuntu package version, architecture, and SHA-256 manifest (157 packages)</summary>

```text
Package: adduser Version: 3.137ubuntu1 Architecture: all	542aff2395eb912a56833e11a78076342779689d8fede7adcf08d1d6b0d0cd8b
Package: adwaita-icon-theme Version: 46.0-1 Architecture: all	530d1991a87ffda50f35f9a6d23712ed04c10b742c3e434097840fed50b1eb4e
Package: at-spi2-common Version: 2.52.0-1build1 Architecture: all	7e05959b067031468f21ae46c6653a6813f1dccd994ef12a0b5f75de0ed346b6
Package: dbus-bin Version: 1.14.10-4ubuntu4.1 Architecture: amd64	0d2c7e425967039594ea099f6128231769e1ed2cc2f6fd7d895fea135c9e1964
Package: dbus-daemon Version: 1.14.10-4ubuntu4.1 Architecture: amd64	785ad36aafc8912300e44a1cea438e28ea36fb94c631f5849408aea85d2ab2bf
Package: dbus-session-bus-common Version: 1.14.10-4ubuntu4.1 Architecture: all	aa88bf2f2107e7d948d0e01001b447fc6cae81b285a6e7daf33dcb62737de62f
Package: dbus-system-bus-common Version: 1.14.10-4ubuntu4.1 Architecture: all	4db2e3e18c6abee1a1d1e2f91289e229dd862e6e1ef00463c5533c612c4f1809
Package: dbus-user-session Version: 1.14.10-4ubuntu4.1 Architecture: amd64	e585b1694b854c3b75bfb39cc4022cafe7b14e44fd435433b613b8fb9919cb41
Package: dbus Version: 1.14.10-4ubuntu4.1 Architecture: amd64	0d59be1393d5b01552edbf16c7b9357c473bf625aa47365ce2e8eef6da1dd2e1
Package: dconf-gsettings-backend Version: 0.40.0-4ubuntu0.1 Architecture: amd64	2e6801f955d5960380b6240be3886162343e770b1c17c919674cbcb3d13be732
Package: dconf-service Version: 0.40.0-4ubuntu0.1 Architecture: amd64	49bf5473474228d48373da639bbe3e77157b03d4467bfa2deb56168c2c03cd81
Package: fontconfig-config Version: 2.15.0-1.1ubuntu2 Architecture: amd64	9df0579cd298a1f2b7a40efd519c849745aecd193a09733cf4d1032284a62bbf
Package: fontconfig Version: 2.15.0-1.1ubuntu2 Architecture: amd64	ddf0aace42b13a5bc38904e848b145b787ea254cddf5213d150254bd654db6e5
Package: fonts-dejavu-core Version: 2.37-8 Architecture: all	40049660c194f3b8a2541fc7369efebb10e9f94bdac836a2f38fafedd10fa73a
Package: fonts-dejavu-mono Version: 2.37-8 Architecture: all	8a599d6553307db7ecb795d2f0e5a301e03234afc75c7358b0ba43466454c89a
Package: gtk-update-icon-cache Version: 3.24.41-4ubuntu1.3 Architecture: amd64	bd59cdd3d08524cfbf291efa5f9bed68916375e89476cc04ac5af515eb10e33c
Package: hicolor-icon-theme Version: 0.17-2 Architecture: all	9000eb98868252261978ff49501c0ace3124cb369c395e9f5015ddc556fe2ba6
Package: humanity-icon-theme Version: 0.6.16 Architecture: all	a9bb76c7eb926bd97e50455a79f266de826a246b2250e1ac4aebcda013dd51c7
Package: libapparmor1 Version: 4.0.1really4.0.1-0ubuntu0.24.04.7 Architecture: amd64	4205351c37f4e813f1ca81b6d59a00071f0f70869e652f4ab9e5ba7e5e895d34
Package: libargon2-1 Version: 0~20190702+dfsg-4build1 Architecture: amd64	e2c50cb5c50ca598585397f4d4ce43e6f0b5e3ddc212c23b30bdb34ca4a61f66
Package: libasound2-data Version: 1.2.11-1ubuntu0.3 Architecture: all	0c26896193480d9dbb055e85e8ddfb9c9f34cf19afee340987cd6311e2886fc1
Package: libasound2t64 Version: 1.2.11-1ubuntu0.3 Architecture: amd64	5d17bfb5683eb99f1be951fe48e1be5bf3c52391913b6b74a8aecf9b4f5e779f
Package: libatk-bridge2.0-0t64 Version: 2.52.0-1build1 Architecture: amd64	22b7d47e3c0f7953a78d3cfd309d1b20771085de9220fbe610c0989b9e808c9b
Package: libatk1.0-0t64 Version: 2.52.0-1build1 Architecture: amd64	42c5d4b00954f17c2c3c4b866844f691eb8b6d57bf08b59203e65f12dc84a4f9
Package: libatspi2.0-0t64 Version: 2.52.0-1build1 Architecture: amd64	d686ace4080eca9c2d6ce8de69392a5ac56c846701894c0858922d4ce341a1d7
Package: libavahi-client3 Version: 0.8-13ubuntu6.2 Architecture: amd64	00609b52ea5a1a9e35fcee4bfba03b6ff743a8015649026cb9109c356762f4d9
Package: libavahi-common-data Version: 0.8-13ubuntu6.2 Architecture: amd64	9a03de928ae470c8d127a22dcb8fb8b582922a8f9406fc3ea879f5e98a02eab4
Package: libavahi-common3 Version: 0.8-13ubuntu6.2 Architecture: amd64	edd1f7a4a21e70504d0872c429600f2e13fdff7775c0952145f1afeacede1c82
Package: libbrotli1 Version: 1.1.0-2build2 Architecture: amd64	74492419b8fda803774b8c9acef6afc5d2f9ff31782635aae212906adae7b277
Package: libbsd0 Version: 0.12.1-1build1.1 Architecture: amd64	f3857b0863ac5cfd4263e9bf6cfb1d4be88d5321e4070d5bc2b62b0949e6c86f
Package: libcairo-gobject2 Version: 1.18.0-3build1 Architecture: amd64	f5483cbe43455fc9e9cba7f0e166bd0a731c02db2902ba562719848f72e8c403
Package: libcairo2 Version: 1.18.0-3build1 Architecture: amd64	96950b306889ff6e4248bb937b0a3b56e72dacf643fe732214ce375f0f6bbb36
Package: libcolord2 Version: 1.4.7-1build2 Architecture: amd64	ef148eb22d566506302a133516594e455e59307aa0dc9874c15a8618268e3355
Package: libcryptsetup12 Version: 2:2.7.0-1ubuntu4.2 Architecture: amd64	f6f6a7f35104da711997b52c92ab91ce1f47dd039d276d5beb797d713447c9f2
Package: libcups2t64 Version: 2.4.7-1.2ubuntu7.14 Architecture: amd64	b815a3d936718614e403035bb4a545e1323305ea624709bff2e9d94c214f9cc4
Package: libdatrie1 Version: 0.2.13-3build1 Architecture: amd64	1ae164e40e413eac4fbd38ce1c6ef9591f7a03278ad9076d71fa29258727f447
Package: libdbus-1-3 Version: 1.14.10-4ubuntu4.1 Architecture: amd64	5d630480f04b4b442300ce847a3fa705ea4d14d80ba6de91f99b51a4e4953b08
Package: libdconf1 Version: 0.40.0-4ubuntu0.1 Architecture: amd64	86bf3825fbaec0d5159c5e6bd88f10ade991a086c821aeef15631c510c6d9904
Package: libdeflate0 Version: 1.19-1build1.1 Architecture: amd64	8e7dfa9e63a9b5058071b84da48bb2e30dea07de169e80fbc759fe0e68639269
Package: libdevmapper1.02.1 Version: 2:1.02.185-3ubuntu3.2 Architecture: amd64	0d43a02b9030a50edbc49c9580f88f01eef2b043baad1c6200672c39643c0efe
Package: libdrm-amdgpu1 Version: 2.4.125-1ubuntu0.1~24.04.2 Architecture: amd64	aa29b7898bae596bf07e55ba178cf8d5ea698fe83688e1b34520d86010251a7a
Package: libdrm-common Version: 2.4.125-1ubuntu0.1~24.04.2 Architecture: all	0b60996bc18fa1545e60a47bb0fe1a7f360ec06fe62ba35fb8d8ba2a54e62166
Package: libdrm-intel1 Version: 2.4.125-1ubuntu0.1~24.04.2 Architecture: amd64	847195765b9ef7e7886677aeae9987aa54e496758b4e6e5a4ed2cb580b252244
Package: libdrm2 Version: 2.4.125-1ubuntu0.1~24.04.2 Architecture: amd64	4d3e858dd57be617c49d87c5ddcdb35df744a4f0a559981256ae2ba174718584
Package: libedit2 Version: 3.1-20230828-1build1 Architecture: amd64	3afbc8ddf835049dd4d88275b9d13680e4c4cdd59e191cbd63424152cc08e00e
Package: libelf1t64 Version: 0.190-1.1ubuntu0.1 Architecture: amd64	b4aee123a37472d213647594b4f7d4a0d13615b0ca2a6fd33926b768d6b232c2
Package: libepoxy0 Version: 1.5.10-1build1 Architecture: amd64	e2e754da4e7fb89021476b093cb5ebe9b1859a688fa37739809e529627d39f1f
Package: libexpat1 Version: 2.6.1-2ubuntu0.4 Architecture: amd64	126a5612e652bdc2edee19ae8fe4308db72b5b3b0a5581bf885b44a093baf3e5
Package: libfdisk1 Version: 2.39.3-9ubuntu6.5 Architecture: amd64	a67f2ed8b8a0323ab3c1e33a8caf81b9bf8ceb1d3586ec5600f40e1406c48f6a
Package: libfontconfig1 Version: 2.15.0-1.1ubuntu2 Architecture: amd64	a2bc05cfef021fdb84285036f98eda5ebec3c4b7a378f5aa2bdba5c4d3d8d586
Package: libfontenc1 Version: 1:1.1.8-1build1 Architecture: amd64	0c5d57211cac9659e7f4c856f5e1922c4a522fe8bcf5776e9707b2811f965702
Package: libfreetype6 Version: 2.13.2+dfsg-1ubuntu0.1 Architecture: amd64	f6937fd8a77e83001dcfd3857d5a5cfbd4caaeb297c7fdb71ec50514843e48af
Package: libfribidi0 Version: 1.0.13-3build1 Architecture: amd64	6cd50259d39ce0dfafee2632c6268538e6a02590e77161a133e9683af346dd1d
Package: libgbm1 Version: 25.2.8-0ubuntu0.24.04.2 Architecture: amd64	1c0b8daf0130d68de428334ce6d1e11bd11f7369bf9099bbc65ac88117336dc3
Package: libgdk-pixbuf-2.0-0 Version: 2.42.10+dfsg-3ubuntu3.3 Architecture: amd64	de84e0d777d9c9e39148d8491338e6b8c62bc2d066867266b1643a7a725a86ba
Package: libgdk-pixbuf2.0-common Version: 2.42.10+dfsg-3ubuntu3.3 Architecture: all	57f24caf1fe608d009933591bdbbf70851a4dbee8b75e9c097f68f57a9a72e07
Package: libgl1-mesa-dri Version: 25.2.8-0ubuntu0.24.04.2 Architecture: amd64	cb46b5d0cdaa9174b7f5362ead3f794d1e3674fcdfb9631950216f5acc9c591d
Package: libgl1 Version: 1.7.0-1build1 Architecture: amd64	e2b2fadeb883f073b566ad1d7874f6702397514d258473e96152a89d4d502a09
Package: libglib2.0-0t64 Version: 2.80.0-6ubuntu3.8 Architecture: amd64	ca4eda0e01c76ba2e5e501e77decce53d98a641234b517eb98fc5d74980f913f
Package: libglvnd0 Version: 1.7.0-1build1 Architecture: amd64	33f5e07c74f73c2bfb44086ae0f9e6da52acd6c104d30ffe8fd768d8253d5e82
Package: libglx-mesa0 Version: 25.2.8-0ubuntu0.24.04.2 Architecture: amd64	83ff69f8d68c073efd1e1ea66e5709562b677d0e3e17b8bcd65850cab887c4f6
Package: libglx0 Version: 1.7.0-1build1 Architecture: amd64	aa4953182f30abde90cb5b072e83d7286b557b325769b8686196a7bd396d8795
Package: libgraphite2-3 Version: 1.3.14-2ubuntu0.24.04.1 Architecture: amd64	2413ea0f6670d6610ee7c2f550551d69fa341f4b9c78e20eb397f0d1bbe914cb
Package: libgssapi-krb5-2 Version: 1.20.1-6ubuntu2.8 Architecture: amd64	6cd99ec16ae12eb465712f950e43eaf03a8d2a6ab24c00178df56470d5343b66
Package: libgtk-3-0t64 Version: 3.24.41-4ubuntu1.3 Architecture: amd64	fd61e308234b12d243160798216d0efcb605d83af185bf3414ac99c2ed5166fc
Package: libgtk-3-common Version: 3.24.41-4ubuntu1.3 Architecture: all	193be16ae51e14f6a209de012e61916761be866c6babaa48d7faa9e641567950
Package: libharfbuzz0b Version: 8.3.0-2build2 Architecture: amd64	d6f7eea2244f98aa0463a056680d4629476bf624767ede301560e24add686b5c
Package: libice6 Version: 2:1.0.10-1build3 Architecture: amd64	ad1edb5303574fee154e487947177e5bd15363aa71b92f78f5a2a7145fdb81c5
Package: libicu74 Version: 74.2-1ubuntu3.1 Architecture: amd64	c9a70989678660eed9a1e904c74fa043da8bec8e2036856fc16e31ced79b04f8
Package: libjbig0 Version: 2.1-6.1ubuntu2 Architecture: amd64	074a760a86213a1286ad60f56394c90885cf45af070626a88c480d90b78c175a
Package: libjpeg-turbo8 Version: 2.1.5-2ubuntu2 Architecture: amd64	f68b5b23bc8a1688fb787d2aed7e2cdf895a73022f6a5025e183162dac4500b2
Package: libjpeg8 Version: 8c-2ubuntu11 Architecture: amd64	4d6ed682972bddf62af158a06d68a0dda497e421423b7ca92fb450b3d9ceaac9
Package: libjson-c5 Version: 0.17-1build1 Architecture: amd64	3ac608d78e9e981e14ba7ceb141a46e3dd3ae8d6dbb3a7ecafd2698fe9930584
Package: libk5crypto3 Version: 1.20.1-6ubuntu2.8 Architecture: amd64	48f689737191cfafaf3c158e9b07d6448f9e6217ad7abbaacc4f96dc95403fa2
Package: libkeyutils1 Version: 1.6.3-3build1 Architecture: amd64	0679f198b0128179e46cdf956fb2022c23c758664c00bc8efa0382d509683a8a
Package: libkmod2 Version: 31+20240202-2ubuntu7.2 Architecture: amd64	a9cbdc424bc0a5c8af3d6445488a48de76df5ff4d76b7dab8aaf88f712358bbc
Package: libkrb5-3 Version: 1.20.1-6ubuntu2.8 Architecture: amd64	63ab8110daea359f55d8135d395de198257acb1f948500c561745addddfece4c
Package: libkrb5support0 Version: 1.20.1-6ubuntu2.8 Architecture: amd64	cee1efc93d4ce4a97db756269824b5a2b90d2cb993cd76102432db60890819fc
Package: liblcms2-2 Version: 2.14-2ubuntu0.1 Architecture: amd64	f48368ce55ac35c4723a158c74f2c48f4398570d6f3207874e3a14e1cdada71d
Package: liblerc4 Version: 4.0.0+ds-4ubuntu2 Architecture: amd64	f1b9dbfa28c56564f6b544f3757f5f3f4d549ce9962f39e897812c23b0a5d0e7
Package: libllvm20 Version: 1:20.1.2-0ubuntu1~24.04.3 Architecture: amd64	f018e71c72c7a4742fb1aff9dc98b45586f42e6ea464d4918af7e3772b7f0658
Package: libnotify4 Version: 0.8.3-1build2 Architecture: amd64	4d856e960ac0ec137e9908b6df0fa295e04c175b0be239450de2243be49524df
Package: libnspr4 Version: 2:4.35-1.1build1 Architecture: amd64	e579e72d091f6c7a13f5a756c31065b15aae5b81840d61b069355aa2283c07b4
Package: libnss3 Version: 2:3.98-1ubuntu0.2 Architecture: amd64	4254f11d782dfb970e78113519a59d440112ed43c4b56f709da0aef81da8651a
Package: libpam-systemd Version: 255.4-1ubuntu8.17 Architecture: amd64	60b50012e97f815686fcd5ff2055c13396d354bbac9974a77e8557346ccebe4e
Package: libpango-1.0-0 Version: 1.52.1+ds-1build1 Architecture: amd64	09dfa5c881ab273ec6bc1830adcefdc407fcd619316e68fb26ca3a23d0fc9f54
Package: libpangocairo-1.0-0 Version: 1.52.1+ds-1build1 Architecture: amd64	98cf5fc9076c2911fe20dfc1e41cd8f686a2bb5d539b1105fab18105b0db1376
Package: libpangoft2-1.0-0 Version: 1.52.1+ds-1build1 Architecture: amd64	cc6bc9d86ef5f329edacaa4895232d8a9b8f95c5cbc77c3c460a3f64cecc213d
Package: libpciaccess0 Version: 0.17-3ubuntu0.24.04.2 Architecture: amd64	c0d818c0ebb2e61f4c0d75f532a8f2e1a48289fe1132f4936c790ed0fa164883
Package: libpixman-1-0 Version: 0.42.2-1build1 Architecture: amd64	d9c2931c4c424615eeab9dd5ae08bfc608b84e803c1d3ccddf319270db213421
Package: libpng16-16t64 Version: 1.6.43-5ubuntu0.6 Architecture: amd64	ac0163f58b8aa52dfdd17cdc10a83f3db084cb91a62686023f5c983d6ff89f8b
Package: libsecret-1-0 Version: 0.21.4-1build3 Architecture: amd64	7d7263ffc92c33042328f99c93433a846e8d9e549876dab18ba18606e6541ecd
Package: libsecret-common Version: 0.21.4-1build3 Architecture: all	41866e9026e451fd25a4cbe887fa56c43219f9f3eda88d1831452ba6d6b8bb5b
Package: libsensors-config Version: 1:3.6.0-9build1 Architecture: all	148103a436fd273dabfe0fe41dc54873df46227b59b2c7ac7339d7e2ddfd033b
Package: libsensors5 Version: 1:3.6.0-9build1 Architecture: amd64	515ed1fbe2bc99926c44fe10b1beb9c3510068c344adda334303098d669ba1e5
Package: libsharpyuv0 Version: 1.3.2-0.4build3 Architecture: amd64	4e1763494e5b9d34192313aa88e38201fc67b3f13ea96ca268d26401a3002791
Package: libsm6 Version: 2:1.2.3-1build3 Architecture: amd64	1d27ebc381b499075a28c504be4d7d424c63b6866354736ac7f8d25813860cdf
Package: libsqlite3-0 Version: 3.45.1-1ubuntu2.7 Architecture: amd64	488511119cad001a00f7e00e597112cf743ccfbd3f7a03c82d66237e1bfd82c8
Package: libsystemd-shared Version: 255.4-1ubuntu8.17 Architecture: amd64	8722a69d99c1a7e6f72c5389e9c6d8053d95d2f5f4fede5dc83118f735812518
Package: libthai-data Version: 0.1.29-2build1 Architecture: all	6035d4a714290b97816bed7f3bf64921c6358ddb84d1dacdddccdc1cf39bffb7
Package: libthai0 Version: 0.1.29-2build1 Architecture: amd64	5ada7045c5f84a4b774e6449151800aa6470e8d01dc9d6124ef4a44ae6af5508
Package: libtiff6 Version: 4.5.1+git230720-4ubuntu2.5 Architecture: amd64	564cdf59dda5e1a826c51b5868ee2785fffa26ee5a7bad95bc5e8c8224c931b8
Package: libunwind8 Version: 1.6.2-3build1.1 Architecture: amd64	825cb8724bb6b3389587e30c03ab3381f56a2422cafe3b1945f37e055c65e942
Package: libvulkan1 Version: 1.3.275.0-1build1 Architecture: amd64	ccf4fe8f4461442f27ea2494c7ae650b60bd396fec2688b0c44a27d66a222f74
Package: libwayland-client0 Version: 1.22.0-2.1build1 Architecture: amd64	6af0aed41d75149bea22fa468f01eb058ffe3e35ef07ff2f13fb88a90387881d
Package: libwayland-cursor0 Version: 1.22.0-2.1build1 Architecture: amd64	29c20da27e8e37c984cf0518635c11cd0441f4cb6a2919a9f6eee1809171d327
Package: libwayland-egl1 Version: 1.22.0-2.1build1 Architecture: amd64	76cd17d2776428e0a0551b8f8fd954eeb3d39ca8ed2f88a38a5e4ce3ce3b2f16
Package: libwebp7 Version: 1.3.2-0.4build3 Architecture: amd64	c86f6439f0bbc531f7199794654231f3b29327d6585958511b958795d51c2484
Package: libx11-6 Version: 2:1.8.7-1build1 Architecture: amd64	397f84347476a3c5786b39f3ff6f0f82866eb3d8be6d2ad3efeadf019efe5b80
Package: libx11-data Version: 2:1.8.7-1build1 Architecture: all	9ae01f7747e7f479394c697c55acf11d3baf139e8e54b7c4adfb55ef9c50de08
Package: libx11-xcb1 Version: 2:1.8.7-1build1 Architecture: amd64	7d0d357e47cd6e1042be34da1d37cea313420b000035e71e855087c8268ab127
Package: libxau6 Version: 1:1.0.9-1build6 Architecture: amd64	e40d29f1d1a62393bacaedebe0da3d9006084152a9f7e5e029293f08ce1c5c80
Package: libxaw7 Version: 2:1.0.14-1build2 Architecture: amd64	2a9cdc82ecf566de76e05394efc604c9cced49e12c28b10e1ca101062b47bac0
Package: libxcb-dri3-0 Version: 1.15-1ubuntu2 Architecture: amd64	3d3d0e95e56ae16010a84883196b8153cba62d6831f9aaee2a68c46f20c7c004
Package: libxcb-glx0 Version: 1.15-1ubuntu2 Architecture: amd64	d07cde26c70fe92df7c327928371e495cc4570fed2d6518758a5db5715cb34c1
Package: libxcb-present0 Version: 1.15-1ubuntu2 Architecture: amd64	2e36e6557e87dbfd64df3143cbf81a4f75c3447501a545b4730511d228b0dd17
Package: libxcb-randr0 Version: 1.15-1ubuntu2 Architecture: amd64	b5cb519823a05a617b543dc9b6a9289b7648b20048244abe021caa706309835a
Package: libxcb-render0 Version: 1.15-1ubuntu2 Architecture: amd64	7d83a0668b1a693b5ea2e496206e8371fb6beda4c54a2b4b3902005915c272c5
Package: libxcb-shm0 Version: 1.15-1ubuntu2 Architecture: amd64	229d1280d459f1ba44c22939d3f9b61d9d20932d9a646b3fe4ce50be4cdf2325
Package: libxcb-sync1 Version: 1.15-1ubuntu2 Architecture: amd64	14286795a258593259923619073e50c15345673befae2733f7b0577b07359aa8
Package: libxcb-xfixes0 Version: 1.15-1ubuntu2 Architecture: amd64	0b2ab64af92a71e3d1a35e3c819880ee28d04bfac81360df68ae8d8e9663ebd2
Package: libxcb1 Version: 1.15-1ubuntu2 Architecture: amd64	e1c6611d11ad7398326f1bf028afc34c3b14c51d917a3426b966ed4b9687fa58
Package: libxcomposite1 Version: 1:0.4.5-1build3 Architecture: amd64	a12d5d9ae70b798cbbf284080eae32b87367561fc9c24492f641b8860ac0f308
Package: libxcursor1 Version: 1:1.2.1-1build1 Architecture: amd64	1a4beee621454e42fe0d2f4e1633611126e43d891f0aec28e4a06744f6e9f04d
Package: libxdamage1 Version: 1:1.1.6-1build1 Architecture: amd64	6891faf325e996ebd28ef53ebb9c043dc96f67327500490affc3e89d366d35ed
Package: libxdmcp6 Version: 1:1.1.3-0ubuntu6 Architecture: amd64	bcd336fce11ce2a45f34d0f95e6980af22529f22147e8f98c156e5cee8ee42bb
Package: libxext6 Version: 2:1.3.4-1build2 Architecture: amd64	45783969a9ece9d7b7b733b8c60981584c53c6bc5ee3b42d295d2f80d1285679
Package: libxfixes3 Version: 1:6.0.0-2build1 Architecture: amd64	0ee1015cccd063249e01c0cd0bf45f513c8ac9a1e5e485070c12e69192455e4a
Package: libxfont2 Version: 1:2.0.6-1+deb13u1build0.24.04.2 Architecture: amd64	e2ed49a20ccff6ffa52719a6cd118538e24c434d9035acd7fb736e7e4f8d6485
Package: libxi6 Version: 2:1.8.1-1build1 Architecture: amd64	0ea7acb5e8a8ce4d6653b30e03a319bfe136db7bfb5ee7cad34c2aa1272ea8d9
Package: libxinerama1 Version: 2:1.1.4-3build1 Architecture: amd64	c8c93a3efdbb06e314f41bf48af4ca3652e4a3826bab5b9e00887660e619f239
Package: libxkbcommon0 Version: 1.6.0-1build1 Architecture: amd64	2b9caeb423efb540296a1cb20b872cc630c23908407ecb5c1c787a617622d664
Package: libxkbfile1 Version: 1:1.1.0-1build4 Architecture: amd64	22ac5b7dca740e606261cebdfaf1a07738e92ad3d123570694374ddb4d5afea2
Package: libxml2 Version: 2.9.14+dfsg-1.3ubuntu3.8 Architecture: amd64	bfd07c01d6e5ab3e327f3ca5819409b1914bbfb3f1a016d53e4dabd5f96143bb
Package: libxmu6 Version: 2:1.1.3-3build2 Architecture: amd64	05c13bce0e9a80139dfa72cc8e7a695b4f7d58b1884423929eade62179d99853
Package: libxmuu1 Version: 2:1.1.3-3build2 Architecture: amd64	27428f8d7b63c38424177d5b3b8b2b89d519956ccfe969927cd0a20cdec977c0
Package: libxpm4 Version: 1:3.5.17-1ubuntu0.24.04.1 Architecture: amd64	f2a2651b67368882030c24dabfdcd459853a3102d602f22c9bb16e685ad772df
Package: libxrandr2 Version: 2:1.5.2-2build1 Architecture: amd64	f2955a5e594f5724b58ad241d9231ea191cb36574a0d5e5ca6b661cd41d6256d
Package: libxrender1 Version: 1:0.9.10-1.1build1 Architecture: amd64	d70bd831aebe8d4834b5dd2ed98df26dd6bd27f1042c47543bd7f66df1ae22ea
Package: libxshmfence1 Version: 1.3-1build5 Architecture: amd64	bc6de4bfaf9050a8ba83d4bcfb114131b081084a7972c03417f8523d52b5c742
Package: libxss1 Version: 1:1.2.3-1build3 Architecture: amd64	0ac60a2cc034ccc4bf4e2f846f38110469d7ae43b47e808d67aafc538bc3695e
Package: libxt6t64 Version: 1:1.2.1-1.2build1 Architecture: amd64	2e4317d5ae1a45e517771ea0b8d93519b5f915ab08983facecd480d64dbd1489
Package: libxtst6 Version: 2:1.2.3-1.1build1 Architecture: amd64	632d74c760f0a4e844c499e7bc1eca4c709aa096fde783a7a6807604aef77545
Package: libxxf86vm1 Version: 1:1.1.4-1build4 Architecture: amd64	48b2799f52b86505d2bc91dcb1bfa6536545635b9849cb693d274d934116a8da
Package: mesa-libgallium Version: 25.2.8-0ubuntu0.24.04.2 Architecture: amd64	e46e42e305339765f51f994543cef2a8bf4a7d9ab7004161db1a6d1f291694de
Package: shared-mime-info Version: 2.4-4 Architecture: amd64	7a774ea291ab99bfa987866c6b68e825531eeea7c8a732dd47d14cf480453fda
Package: systemd-dev Version: 255.4-1ubuntu8.17 Architecture: all	70b863bd1188f49a3ba0ccde5355abd6611a8a5dcd8b92addf25c8036f22556d
Package: systemd-sysv Version: 255.4-1ubuntu8.17 Architecture: amd64	000c90facb7f0f33a3c771ac8cf58221f80239184fb9e24fe78086ffea6e2cf9
Package: systemd Version: 255.4-1ubuntu8.17 Architecture: amd64	250345b73e42a97ee71cae7c1e470897dfad3c639eb0a7aafe4f9a3a29871cb2
Package: ubuntu-mono Version: 24.04-0ubuntu1 Architecture: all	3a58114f10170b253d2a574db854fa1c4399565888c78f28bbe53ce889894cdb
Package: x11-common Version: 1:7.7+23ubuntu3 Architecture: all	b545fa5196dd7467ba3770d6ce575abcd7941071961ea9f7535a319d9d20fd46
Package: x11-xkb-utils Version: 7.7+8build2 Architecture: amd64	69df4191e165f8b89d067f6234fcce89a4923d27d1e66dd92e6d63d6da93cdb0
Package: xauth Version: 1:1.1.2-1build1 Architecture: amd64	037e70571f39384e28c4ad0db07b44de11f0911ee6c02445560979b417012650
Package: xdg-utils Version: 1.1.3-4.1ubuntu3 Architecture: all	499ad8b146f8d6f092ada8f878e7c6d93239caf29ce92c1381fe94e314169986
Package: xkb-data Version: 2.41-2ubuntu1.1 Architecture: all	e99f2a0b4c56cdf35dec489e56b6a8418826cab8ce6306ec6d0bd98e2cd0014a
Package: xserver-common Version: 2:21.1.12-1ubuntu1.6 Architecture: all	5dd824cb0c0f59776cd60cd284c53df0e7485b5b09b79350071f9aef524e5eea
Package: xvfb Version: 2:21.1.12-1ubuntu1.6 Architecture: amd64	9188c2ab6394dfe6aa53f782f6bfa22b7eb6febfe51a48e0e36af35cd2f64307
```

</details>

## Fresh validation

| Command or inspection | Result |
| --- | --- |
| `node --test scripts/docs/check-public-diagrams.test.mjs` | GREEN, 80 tests |
| `pnpm docs:public:check` | GREEN, 106 files, 0 errors |
| `pnpm docs:public:diagrams` | GREEN, 4 files, 0 errors, 0 warnings |
| `pnpm docs:check` | GREEN, 12/12 surfaces; only inherited baselined warnings |
| `pnpm docs:owed` | Exit 0 in its advisory mode; current freshness effects recorded below |
| `pnpm docs:index:check` | GREEN, 6 files, 0 errors, 0 warnings |
| `pnpm format:check` | GREEN, 1,707 files |
| `pnpm aps:active-lint` | GREEN, 143 files |
| `pnpm aps:index:check` | GREEN |
| `pnpm aps:drift --json` | GREEN, 0 findings |
| pinned-base sidebar ID/order assertion | GREEN, 47/1/16/7 unchanged |
| `pnpm --filter @eddacraft/anvil-docs-private build` | GREEN |
| `pnpm --filter @eddacraft/docs-public build` | GREEN |
| `pnpm --filter @eddacraft/docs-shell build` | GREEN, 9/9 static pages |
| `git diff --check` | GREEN |

On the pinned starting receipt, both Docusaurus builds emitted a non-fatal
image-dimension discovery warning for the authentic SVGs. Build readback
confirmed that the hashed SVGs were copied into each output and referenced from
generated HTML with the intended alt text; rsvg read them at 944×299 and
1058×330 respectively. After rebasing onto current `origin/main`, all three
builds passed again and the image-dimension warning no longer appeared. The
private build retains its inherited `/anvil/` shell-route warnings.

## Freshness effects after rebase

`pnpm docs:owed` remains advisory until DOCRB-009 and exited 0. Its seven gating
direct file-level findings and one baselined direct finding caused by the
approved normaliser and inventory note were closed after manual compatibility
review. The diagram guide now names the exact normalisation behaviour, while the
semantically unchanged governance guide records its current review anchor. The
AICON-owned guide index and the DOCRB- and DOCGOV-owned architecture, tags,
adapter, reviews, and top-level docs consumers record compatible discovery,
catalogue, and placement reviews. No prose, authority, navigation, baseline, or
generated index changed in those five downstream consumers. The two owning-page
references also move the
broad `docs/public/**` upstream for `docs/architecture/docs-delivery.md`,
which remains an advisory glob-granularity finding because the delivery flow
and topology are unaffected. No direct file-level freshness edge remains.

<div align="center">

<img src="public/watchbridge.svg" width="104" alt="WatchBridge" />

# WatchBridge

**Prepare o modo desenvolvedor do Apple Watch e instale apps com Watch Companion no relógio — pelo Windows.**

[English](README.en.md) · [简体中文](README.md) · [한국어](README.ko.md) · [繁體中文](README.zh-TW.md) · [日本語](README.ja.md) · [Español](README.es.md) · [Français](README.fr.md) · [Deutsch](README.de.md) · [Português](README.pt.md)

[![Build Windows](https://img.shields.io/github/actions/workflow/status/AdminZachary/iloader-for-windows/windows.yml?branch=main&label=windows&logo=github)](https://github.com/AdminZachary/iloader-for-windows/actions/workflows/windows.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
![Platform](https://img.shields.io/badge/platform-Windows%2010%20%2F%2011-0078D4)
![Status](https://img.shields.io/badge/hardware%20validated-2026--11--12-brightgreen)
![Version](https://img.shields.io/badge/version-0.1.0--alpha.3-lightgrey)

</div>

> [!IMPORTANT]
> O WatchBridge é uma derivação comunitária independente do [fork do iLoader de AndreySemjonov](https://github.com/AndreySemjonov/iloader); o projeto original é o [nab138/iLoader](https://github.com/nab138/iloader).
> **Não é uma versão oficial da Apple nem de qualquer projeto upstream, e não implica endosso.** Relate problemas neste repositório, não no upstream.

> [!NOTE]
> O fluxo completo foi validado em um iPhone 17 Pro + Apple Watch Ultra 2 e em um iPhone 13 Pro + Apple Watch Series 7:
> preparação do modo desenvolvedor → assinatura → instalação no telefone → instalação no relógio → abertura do app.

## O que é

Instalar um app iOS assinado por conta própria (como o SideStore faz) exige três coisas: assinatura com Apple ID, registro do dispositivo de destino como dispositivo de desenvolvimento e modo desenvolvedor ativado no dispositivo. A cadeia oficial é o Xcode, que só existe no macOS.

O iLoader refez metade disso no Windows, em Rust. **O WatchBridge acrescenta a outra metade**: pega o `Watch/*.app` dentro do app instalado no iPhone e o coloca no Apple Watch pareado; e, antes disso, faz o interruptor de modo desenvolvedor do relógio aparecer. Sem instalar Python e sem linha de comando.

O caminho até o dispositivo é indireto: o Windows não enxerga o relógio, então o programa usa o iPhone como roteador, encaminha uma porta para o relógio e faz o pareamento, a leitura de estado e a instalação dentro dessa sessão encaminhada.

## Recursos

| | Recurso | Detalhe |
|---|---|---|
| 🛠 | **Preparação do modo desenvolvedor** | Parear o relógio → solicitar o interruptor → ler o estado. Apenas `action 0` é enviado; **ativar, reiniciar e confirmar é sempre feito por você no relógio** |
| 📦 | **Instalação do Companion** | Verifica se o IPA contém o host do iPhone e `Watch/*.app`, reassina, instala no telefone e depois instala o app no relógio |
| 📶 | **Wi-Fi / RSD** | Configure o Wi-Fi uma vez por USB e depois instale e renove sem fio; a preparação do modo desenvolvedor continua exigindo USB |
| ♻️ | **Renovação automática** | Reassina antes de expirar os 7 dias, com controles na bandeja e início automático opcional |
| 🔐 | **Conta e certificados** | Login com Apple ID (identidade do dispositivo via anisette), consulta/revogação de certificados, gestão de App IDs, suporte a keyring |
| 🔗 | **Arquivo de pareamento** | Importar/exportar pairing file, reparar o pareamento por Wi-Fi |
| 🌍 | **Interface multilíngue** | 28 idiomas, incluindo chinês simplificado e tradicional |

## Fluxo de uso

```text
① Conecte o iPhone pareado via USB   ② Pareie o relógio e confirme a confiança nele
   Selecione o dispositivo              Peça o interruptor «Modo desenvolvedor»
   ↓                                    ↓
   Ative no relógio → reinicie → confirme → clique em «Verificar novamente»
   ↓
③ Entre com o Apple ID → escolha o IPA original → assine e instale
   ↓
   Abra o app você mesmo no relógio e confirme que ele abre
```

> [!NOTE]
> O programa **não** ativa o modo desenvolvedor, não reinicia o relógio e não confirma a confiança por você. Os registros de confiança existem apenas na memória: ao reiniciar o programa é preciso criar uma nova sessão.

## Requisitos

- Windows 10 22H2 / Windows 11 (x86_64)
- **Apple Mobile Device Service**: incluso no [iTunes](https://www.apple.com/itunes/) ou no app Apple Devices da Microsoft Store. Sem ele, nem USB nem Wi-Fi funcionam
- Runtime do WebView2 (já incluído no Windows 11)
- Um iPhone já pareado com o relógio de destino; a preparação exige cabo

## Instalação e execução

Baixe `watchbridge-windows.zip` em [Releases](https://github.com/AdminZachary/iloader-for-windows/releases/latest), descompacte e execute `WatchBridge.exe`.

O arquivo contém:

| Arquivo | Conteúdo |
|---|---|
| `WatchBridge.exe` | O aplicativo, **sem assinatura**; o SmartScreen avisa na primeira execução |
| `BUILD.json` | Commit de origem, se a árvore estava modificada, hashes dos lockfiles, versões das ferramentas e SHA-256 do exe |
| `SHA256SUMS.txt` | Somas de verificação dos arquivos acima |
| `LICENSES.zip` | Textos de licenças de terceiros e inventário de dependências (`inventory.json`) |

## Compilar a partir do código

As dependências e o procedimento completo estão em [BUILDING.md](BUILDING.md). O GitHub Actions compila, executa os testes unitários Rust de `iloader` e `isideload` e gera o artefato acima.

```powershell
# Somente o frontend
bun install --frozen-lockfile --ignore-scripts
bun run build

# Compilação completa para Windows (testes + exe + somas de verificação)
.\scripts\Build-Windows.ps1 -BuildExecutable
```

Adicione `-SkipTests` para uma compilação local rápida sem testes. A compilação precisa ser nativa em Windows + MSVC; compilação cruzada não é suportada.

## Compatibilidade e limites conhecidos

| Limite | Detalhe |
|---|---|
| **Um único relógio** | Se o iPhone tiver vários relógios pareados, a preparação se recusa em vez de adivinhar. O protocolo de encaminhamento não tem parâmetro para «escolher o relógio», e agir no relógio errado é pior do que falhar |
| **Sem pacotes só de Watch** | O IPA original precisa conter o host do iPhone e `Watch/*.app` |
| **App Group** | Apps que exigem um Watch App Group ainda podem ser afetados pelo comportamento do upstream que ignora essa associação |
| **Versões do sistema** | O fluxo do relógio vem da implementação experimental do xtool; cada combinação de iOS/watchOS precisa de validação em hardware |
| **Ferramenta não oficial** | Sem jailbreak, sem burlar a assinatura da Apple, sem certificados falsificados |

## Perguntas frequentes

<details>
<summary><b><code>fail to connect to usbmuxd device socket: io error</code></b></summary>

No Windows, o usbmuxd é fornecido pelo Apple Mobile Device Service e escuta em `127.0.0.1:27015`. Esse erro significa que nada responde nessa porta — normalmente o serviço não está instalado ou está parado.

```powershell
Get-Service "Apple Mobile Device Service"      # deve existir e estar Running
Test-NetConnection 127.0.0.1 -Port 27015       # deve ser True
$env:USBMUXD_SOCKET_ADDRESS                    # deve estar vazio
```

Se o serviço não existir, instale o iTunes ou o app Apple Devices. Se o endereço foi sobrescrito, remova essa variável de ambiente. O programa **não inclui** usbmuxd; depende totalmente do driver e do serviço da Apple.
</details>

<details>
<summary><b>«Não é possível ler as informações do relógio»</b></summary>

Versões antigas consultavam o nome do relógio no registro de pareamento do iPhone antes de parear; em versões recentes do iOS essa consulta não retorna nada. A versão atual estabelece confiança com o relógio primeiro e lê a identidade na própria sessão do relógio. Se ainda falhar, abra «Configurações e logs → Ver logs» e confira o valor `kind=` de `watch step failed`: `timed out` significa que o relógio não respondeu; `NotFound` / `UnexpectedResponse` indicam diferença de protocolo.
</details>

<details>
<summary><b>«Vários relógios pareados encontrados»</b></summary>

Veja o limite de um único relógio. Confirme o relógio de destino no app Watch do iPhone; desligar o outro relógio ou afastá-lo **não** o remove do registro de pareamento.
</details>

## Como contribuir

- Regras de contribuição: [CONTRIBUTING.md](CONTRIBUTING.md)
- O upstream proíbe explicitamente contribuições geradas por IA, então **este repositório nunca envia PRs upstream**. Mantenha as mudanças neste fork
- Nunca anexe Apple IDs, senhas, chaves privadas de certificados, registros de pareamento ou logs de dispositivo sem anonimização

## Licença e créditos

O código do projeto é distribuído sob MIT ([LICENSE](LICENSE)); as condições de marca estão em [LICENSE-BRANDING](LICENSE-BRANDING).

> A biblioteca de assinatura na cadeia de dependências (`apple-codesign-quick` do `apple-crates`) não declara licença; os termos ainda estão sendo esclarecidos. Veja [docs/LICENSE_REVIEW.md](docs/LICENSE_REVIEW.md).

Fontes e atribuição: [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md), [vendor/isideload/PROVENANCE.md](vendor/isideload/PROVENANCE.md).

| Fonte | Contribuição |
|---|---|
| [nab138/iLoader](https://github.com/nab138/iloader) | Aplicativo original, assinatura e pipeline de instalação |
| [AndreySemjonov/iloader](https://github.com/AndreySemjonov/iloader) | Ponto de partida deste repositório: Wi-Fi/RSD e renovação automática |
| [oacevedo/xtool](https://github.com/oacevedo/xtool/tree/feat/experimental-watchos) | Fluxo de preparação do modo desenvolvedor (reescrito nativamente em Rust aqui, sem Python) |
| Data_C0re_ (Rzbck) | Assinatura, registro e instalação do Watch Companion |
| idevice / isideload e seus autores upstream | Comunicação com dispositivos e implementação da assinatura |

## Aviso legal

Este projeto não tem vínculo com a Apple Inc. Apple, iPhone e Apple Watch são marcas da Apple Inc.

Apenas para aprendizado, pesquisa e desenvolvimento pessoal; cada usuário deve verificar a legislação de sua jurisdição e os termos de licença dos apps que instala. Os autores não se responsabilizam por danos ao dispositivo, perda de dados, restrições de conta ou revogação de certificados.

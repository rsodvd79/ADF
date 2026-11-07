# ADF

ADF ora include un'applicazione **Rust + eframe/egui**, con progetti XML
versionati e **Rhai incorporato**. Non richiede .NET, componenti COM legacy o un interprete
esterno. Target: **Windows x64** (DirectX 12) e **macOS Apple Silicon** (Metal).

## Avvio della versione Rust

Serve Rust stabile, almeno 1.92, con gli strumenti di compilazione della
piattaforma: Xcode Command Line Tools su macOS, Visual Studio Build Tools
con C++/Windows SDK su Windows.

```bash
cargo run --locked
cargo run --locked -- "examples/ciao.xml"
cargo build --release --locked
```

L'eseguibile è `target/release/adf` oppure `target/release/adf.exe`.
Un percorso da CLI apre il documento nell'editor; **Run** avvia gli script.
Senza argomenti si apre un progetto vuoto, non si chiude l'applicazione.

## Editor e funzionalità

- **XML**: modifica il sorgente e usa **Apply / validate**. Un XML non valido
  non sostituisce l'ultimo modello valido. Il designer è bloccato finché le
  modifiche testuali non sono applicate o scartate.
- **Designer**: palette trascinabile, albero dei controlli, proprietà/eventi,
  duplicazione, eliminazione e undo/redo. Nel layout assoluto trascina per
  spostare e usa la maniglia in basso a destra per ridimensionare; nei layout
  adattivi trascina su un altro controllo per riordinare.
- **Preview** usa lo stesso rendering dell'esecuzione, senza avviare script.
  Ogni `FORM` in esecuzione è una finestra nativa separata.
- **New / Open / Save / Save as / Reload**, protezione delle modifiche non
  salvate, avviso sui file cambiati esternamente, tema sistema/chiaro/scuro
  e dieci progetti recenti.
- **Run / Stop**: l'esecuzione usa una copia del progetto. Le modifiche
  nell'editor si applicano al successivo Run, non al runtime già attivo.
  Chiudere la finestra iniziale termina il progetto ma lascia aperto l'editor.

Le modifiche visuali normalizzano l'XML: non preservano commenti,
formattazione o delimitatori CDATA, ma preservano il testo degli script.
Undo/redo visuale conserva al massimo 32 modifiche; un trascinamento conta
come una sola modifica. Le stringhe Rhai che referenziano controlli devono
essere riviste dopo rinomina o duplicazione.

## XML v2

La radice è `<ADF VERSION="2" SCRIPT_LANGUAGE="rhai">`; restano i concetti
`PROPERTY/STARTCONTROL`, `MODULO` e `CONTROLS/CONTROL`.
`NAME` identifica un controllo nel progetto: è univoco e case-sensitive.
`STARTCONTROL` deve referenziare un FORM alla radice.

| TYPE | Dati/eventi |
|---|---|
| FORM | TEXT è il titolo; contiene widget; LAYOUT può essere ABSOLUTE, VERTICAL o HORIZONTAL |
| BUTTON | TEXT, evento CLICK |
| LABEL | TEXT |
| TEXTBOX | TEXT è il valore iniziale, evento CHANGE |
| CHECKBOX | TEXT è l'etichetta, CHECKED è booleano, evento CHANGE |
| COMBOBOX | OPTIONS/OPTION, SELECTED è un indice zero-based, evento CHANGE |

WIDTH/HEIGHT e TOP/LEFT sono punti logici, non dimensioni esterne
Windows Forms. Il layout ABSOLUTE conserva le coordinate; VERTICAL e
HORIZONTAL usano ordine, PADDING, SPACING e il flag EXPAND dei widget.
Solo FORM può avere figli; i FORM non possono essere annidati.

Esempi:

- `examples/ciao.xml`: conversione del progetto originale. Premendo OK
  appare DVD/CIAO; dopo la conferma il pulsante diventa `SOMMA = 2` e
  il titolo della finestra diventa `ADF`.
- `examples/showcase.xml`: tutti i controlli, layout adattivi, CHANGE,
  conferma Sì/No, stato persistente e una seconda finestra.

## API Rhai

MODULO viene inizializzato una volta per Run; le sue funzioni sono disponibili
a tutti i FORM. CLICK/CHANGE contengono corpi Rhai, non dichiarazioni
`Sub nome_Click`. Gli handler sono compilati prima dell'esecuzione ed
eseguiti uno alla volta, non ad ogni frame grafico.

| Funzione | Comportamento |
|---|---|
| `get_text(name)`, `set_text(name, text)` | Leggere/modificare testo o titolo |
| `get_value(name)`, `set_value(name, value)` | Stringa per TEXTBOX, bool per CHECKBOX, indice intero per COMBOBOX |
| `show_form(name)`, `close_form(name)` | Aprire/nascondere una finestra di progetto |
| `message(title, text)` | Dialogo OK; blocca Rhai e restituisce `"ok"` |
| `confirm(title, text, "ok_cancel")` | Restituisce `"ok"` o `"cancel"` |
| `confirm(title, text, "yes_no")` | Restituisce `"yes"` o `"no"` |
| `get_state(key)`, `set_state(key, value)` | Stato di progetto condiviso, anche dentro funzioni; chiavi assenti restituiscono `()` |
| `print(text)` | Diagnostica; in caso di saturazione i messaggi vengono scartati |

Le variabili top-level di MODULO persistono tra gli handler, ma le funzioni
Rhai non catturano implicitamente le variabili globali: per lo stato
condiviso dentro funzioni usa `get_state`/`set_state`.
Le variabili locali introdotte da un handler vengono scartate alla fine.
Le modifiche programmatiche non generano CHANGE ricorsivi. Un errore
interrompe l'handler, mantenendo le modifiche già effettuate.

I dialoghi bloccano lo script, **non il thread GUI**. Le finestre del
progetto non accettano nuove interazioni durante l'evento/dialogo;
l'editor rimane utilizzabile. Stop, reload e chiusura durante un'attesa
annullano lo script senza eseguire le istruzioni successive.

## Limiti e protezioni

Per inizializzazione/evento: **100.000 operazioni** e **un secondo di
esecuzione attiva**. Il tempo di attesa della risposta a un dialogo è escluso
dal timeout; il contatore operazioni non viene azzerato dal dialogo.
Stop e timeout sono cooperativi.

Limiti finiti: XML UTF-8 5 MiB, profondità XML 32, massimo 1.000 controlli,
testo/script/stringhe 1 MiB, array/mappe e opzioni fino a 10.000 elementi,
64 livelli di chiamata, profondità espressioni 64/32, 256 variabili di scope,
1.024 funzioni e 256 chiavi di stato (128 byte ciascuna, valori scalari).
Geometria finita in 0..16384, con dimensioni positive.

DTD, entità esterne, importazioni Rhai da file ed `eval` sono disabilitati.
Gli script non hanno API per file, rete o processi. Queste sono protezioni
applicative: **non** isolamento del processo o garanzia sulla memoria totale.
I salvataggi validano prima il sorgente e sostituiscono il file tramite
temporaneo nella stessa directory; un errore non elimina il file precedente.

## Migrazione da VBScript

**Import legacy** apre la struttura XML e preserva gli script originali,
ma non li esegue. Gli XML senza versione sono considerati v1/VBScript.
Non esiste una traduzione automatica generale: converti MODULO e ogni
evento, poi usa **Mark scripts converted** o modifica gli attributi della
radice a v2/Rhai. Salva con un nuovo nome per conservare l'originale.

```text
VBScript: Function SOMMA(X1, X2) ... SOMMA = X1 + X2 ... End Function
Rhai:     fn SOMMA(x1, x2) { x1 + x2 }

VBScript: MsgBox "CIAO", vbOKOnly, "DVD"
Rhai:     message("DVD", "CIAO");

VBScript: ok.text = "SOMMA = " & SOMMA(1,1)
Rhai:     set_text("OK", "SOMMA = " + SOMMA(1,1).to_string());
```

Prestare attenzione a maiuscole/minuscole degli ID, conversioni dei tipi e
valori di ritorno dei dialoghi. Non vengono importati oggetti Windows/COM.
Gerarchie, tipi o campi non supportati producono errori, non perdite silenziose.

## Verifiche e distribuzione

```bash
cargo fmt --all -- --check
cargo test --all-features --locked
cargo clippy --all-targets --all-features --locked -- -D warnings
cargo build --release --locked
```

Per uno smoke test con finestre **reali**, serve una sessione desktop e
un backend grafico funzionante:

```bash
cargo run --locked --features native-smoke --example native_smoke
```

Il test nativo renderizza CIAO e showcase, conferma i dialoghi tramite il
protocollo del worker e verifica risultati, CHANGE, viewport e Stop.
I test egui headless verificano separatamente click, blocco durante eventi
e drag-and-drop tramite eventi reali di puntatore. Non sostituiscono una
verifica manuale di focus, screen reader e DPI su ciascun sistema.

La CI aggiunta esegue fmt/test/clippy/build su Windows x64 e macOS ARM64
e produce artefatti. Su macOS, dopo il build release:

```bash
bash scripts/package-macos.sh
open target/bundle/ADF.app
```

Il bundle e gli esempi sono inclusi in `target/bundle/adf-macos-arm64.zip`.
Gli artefatti non sono firmati con certificato distributivo né notarizzati.
La pubblicazione richiede una scelta di licenza e, per distribuzione
firmata, certificati del proprietario.

Il progetto VB.NET resta temporaneamente nel repository come riferimento:
va ritirato solo dopo la verifica della sostituzione anche su Windows.

## Migliorie successive proposte

Hot reload automatico, evidenziazione/completamento Rhai, contenitori e
tabelle, CLI headless ed esportazione di launcher dedicati. Linux, altre
architetture e accesso al sistema con permessi espliciti sono fuori dal
perimetro attuale. Non sono inclusi debugger, plugin, cloud o telemetria.

<details>
<summary>Riferimento della versione legacy VB.NET/Windows Forms</summary>

Programma per creare/automatizzare programmi e logiche partendo da un file XML e script VBScript (VBS), con interfaccia Windows Forms in VB.NET.

## Descrizione

ADF è un progetto Visual Basic .NET che:
- legge definizioni/istruzioni da file XML;
- esegue script VBScript per implementare la logica dinamica;
- utilizza espressioni regolari tramite il motore VBScript RegExp;
- espone un’interfaccia grafica Windows Forms per interagire con i file e mostrare i risultati.

Il repository include un esempio di definizione (`CIAO.XML`) e le librerie di interoperabilità necessarie.

## Funzionalità principali

- Definizione dei flussi/operazioni tramite file XML.
- Esecuzione di script VBS attraverso Microsoft Script Control.
- Supporto a espressioni regolari tramite “VBScript_RegExp_55”.
- Applicazione desktop Windows Forms (VB.NET).
- Configurazione tramite `app.config`.

## Struttura del progetto

- Soluzione
  - [ADF.sln](https://github.com/rsodvd79/ADF/blob/master/ADF.sln)

- Progetto VB.NET (Windows Forms)
  - [ADF/ADF.vbproj](https://github.com/rsodvd79/ADF/blob/master/ADF/ADF.vbproj) — file di progetto.
  - [ADF/adf.vb](https://github.com/rsodvd79/ADF/blob/master/ADF/adf.vb) — form principale (logica).
  - [ADF/adf.Designer.vb](https://github.com/rsodvd79/ADF/blob/master/ADF/adf.Designer.vb) — designer del form.
  - [ADF/adf.resx](https://github.com/rsodvd79/ADF/blob/master/ADF/adf.resx) — risorse del form.
  - [ADF/ClassADF.vb](https://github.com/rsodvd79/ADF/blob/master/ADF/ClassADF.vb) — classe core per la gestione di XML/VBS.
  - [ADF/app.config](https://github.com/rsodvd79/ADF/blob/master/ADF/app.config) — configurazione dell’app.

- Esempi/risorse
  - [ADF/CIAO.XML](https://github.com/rsodvd79/ADF/blob/master/ADF/CIAO.XML) — esempio di definizione XML.

- Interoperabilità (COM interop)
  - [ADF/Interop.MSScriptControl.dll](https://github.com/rsodvd79/ADF/blob/master/ADF/Interop.MSScriptControl.dll)
  - [ADF/Interop.Scripting.dll](https://github.com/rsodvd79/ADF/blob/master/ADF/Interop.Scripting.dll)
  - [ADF/Interop.VBScript_RegExp_55.dll](https://github.com/rsodvd79/ADF/blob/master/ADF/Interop.VBScript_RegExp_55.dll)

## Prerequisiti

- Windows.
- .NET Framework (versione target come da progetto VB.NET).
- Visual Studio (edizione con supporto a Visual Basic .NET).
- Componenti COM disponibili nel sistema:
  - Microsoft Script Control 1.0 (MSScriptControl).
  - Microsoft VBScript Regular Expressions 5.5.
  - Microsoft Scripting Runtime.
- Consigliato compilare come x86 se si usa MSScriptControl (32 bit).

Se i riferimenti alle DLL Interop non si risolvono automaticamente, rimuoverli e ri-aggiungere i riferimenti COM corrispondenti in Visual Studio.

## Come iniziare

1. Clona il repository:
   ```bash
   git clone https://github.com/rsodvd79/ADF.git
   ```
2. Apri la soluzione in Visual Studio: `ADF.sln`.
3. Verifica i riferimenti alle DLL di interoperabilità (COM) e l’architettura di compilazione (preferibilmente x86).
4. Compila ed esegui il progetto `ADF`.

## Uso

- Avvia l’applicazione e carica un file XML di definizione (puoi partire da `ADF/CIAO.XML`) per vedere come vengono interpretate le istruzioni e gli script.
- Adatta o crea nuovi file XML e script VBS per modellare la logica desiderata.
- Configura eventuali parametri in `app.config` se necessario.

## Note

- Le DLL Interop incluse servono a facilitare l’uso di componenti COM da .NET. Su alcuni sistemi potrebbe essere necessario registrare i componenti COM o installarli se mancanti.
- Se incontri problemi con MSScriptControl su sistemi a 64 bit, assicurati di:
  - usare il target x86;
  - avere il componente COM disponibile sul sistema.

## Stato del progetto

Codice originario conservato come riferimento durante la verifica della migrazione Rust.

## Licenza

Non è presente un file di licenza. Se intendi usare, modificare o distribuire il software, contatta l’autore o aggiungi una licenza appropriata al repository.

## Autore

- [@rsodvd79](https://github.com/rsodvd79)

</details>

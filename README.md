# ADF

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

Repository senza licenza dichiarata e senza issue/roadmap pubbliche. È un esempio/prototipo utile per mostrare come integrare XML + VBScript in un’app VB.NET.

## Licenza

Non è presente un file di licenza. Se intendi usare, modificare o distribuire il software, contatta l’autore o aggiungi una licenza appropriata al repository.

## Autore

- [@rsodvd79](https://github.com/rsodvd79)

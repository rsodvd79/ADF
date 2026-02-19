# ADF

Programma per creare/automatizzare programmi e logiche partendo da un file XML e script VBScript (VBS), con interfaccia Windows Forms in C#.

## Descrizione

ADF è un progetto C# .NET 8 che:
- legge definizioni/istruzioni da file XML;
- esegue script VBScript per implementare la logica dinamica;
- utilizza espressioni regolari tramite il motore VBScript RegExp;
- espone un'interfaccia grafica Windows Forms per interagire con i file e mostrare i risultati.

Il repository include un esempio di definizione (`CIAO.XML`) e le librerie di interoperabilità necessarie.

## Funzionalità principali

- Definizione dei flussi/operazioni tramite file XML.
- Esecuzione di script VBS attraverso Microsoft Script Control.
- Supporto a espressioni regolari tramite "VBScript_RegExp_55".
- Applicazione desktop Windows Forms (C#).

## Struttura del progetto

- Soluzione
  - [ADF.sln](https://github.com/rsodvd79/ADF/blob/master/ADF.sln)

- Progetto C# .NET 8 (Windows Forms)
  - [ADF/ADF.csproj](https://github.com/rsodvd79/ADF/blob/master/ADF/ADF.csproj) — file di progetto.
  - [ADF/Program.cs](https://github.com/rsodvd79/ADF/blob/master/ADF/Program.cs) — punto di ingresso dell'applicazione.
  - [ADF/AdfForm.cs](https://github.com/rsodvd79/ADF/blob/master/ADF/AdfForm.cs) — form principale (logica).
  - [ADF/AdfForm.Designer.cs](https://github.com/rsodvd79/ADF/blob/master/ADF/AdfForm.Designer.cs) — designer del form.
  - [ADF/ClassADF.cs](https://github.com/rsodvd79/ADF/blob/master/ADF/ClassADF.cs) — classe core per la gestione di XML/VBS.

- Esempi/risorse
  - [ADF/CIAO.XML](https://github.com/rsodvd79/ADF/blob/master/ADF/CIAO.XML) — esempio di definizione XML.

- Interoperabilità (COM interop)
  - [ADF/Interop.MSScriptControl.dll](https://github.com/rsodvd79/ADF/blob/master/ADF/Interop.MSScriptControl.dll)
  - [ADF/Interop.Scripting.dll](https://github.com/rsodvd79/ADF/blob/master/ADF/Interop.Scripting.dll)
  - [ADF/Interop.VBScript_RegExp_55.dll](https://github.com/rsodvd79/ADF/blob/master/ADF/Interop.VBScript_RegExp_55.dll)

## Prerequisiti

- Windows.
- .NET 8 SDK o Runtime.
- Visual Studio 2022 (o versione con supporto a .NET 8 e C#).
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
3. Verifica i riferimenti alle DLL di interoperabilità (COM) e l'architettura di compilazione (preferibilmente x86).
4. Compila ed esegui il progetto `ADF`.

In alternativa, da riga di comando:
```bash
dotnet build ADF/ADF.csproj
dotnet run --project ADF/ADF.csproj
```

## Uso

- Avvia l'applicazione e carica un file XML di definizione (puoi partire da `ADF/CIAO.XML`) per vedere come vengono interpretate le istruzioni e gli script.
- Adatta o crea nuovi file XML e script VBS per modellare la logica desiderata.

## Note

- Le DLL Interop incluse servono a facilitare l'uso di componenti COM da .NET. Su alcuni sistemi potrebbe essere necessario registrare i componenti COM o installarli se mancanti.
- Se incontri problemi con MSScriptControl su sistemi a 64 bit, assicurati di:
  - usare il target x86;
  - avere il componente COM disponibile sul sistema.

## Stato del progetto

Repository senza licenza dichiarata e senza issue/roadmap pubbliche. È un esempio/prototipo utile per mostrare come integrare XML + VBScript in un'app C# .NET 8.

## Licenza

Non è presente un file di licenza. Se intendi usare, modificare o distribuire il software, contatta l'autore o aggiungi una licenza appropriata al repository.

## Autore

- [@rsodvd79](https://github.com/rsodvd79)


Public Class ADF

    Dim scrMain As New MSScriptControl.ScriptControlClass

    Public Sub New()

        ' La chiamata è richiesta dalla finestra di progettazione.
        InitializeComponent()

        ' Aggiungere le eventuali istruzioni di inizializzazione dopo la chiamata a InitializeComponent().
        AddHandler AppDomain.CurrentDomain.UnhandledException,
        Sub(sender As Object, args As UnhandledExceptionEventArgs)
            Try
                MsgBox(DirectCast(args.ExceptionObject, Exception).Message, MsgBoxStyle.Critical, "ADF")
            Catch ex As Exception
                ' ignora
            End Try
        End Sub

    End Sub

    Private Sub btnOk_Click(sender As System.Object, e As System.EventArgs) Handles btnOk.Click
        Me.Close()
    End Sub

    Private Sub ADF_Load(sender As Object, e As System.EventArgs) Handles Me.Load

        Dim strFL As String = Command().Trim

        scrMain.Language = "VBScript"
        scrMain.AllowUI = True

        If Not strFL.Equals(String.Empty) AndAlso System.IO.File.Exists(strFL) Then

            Dim xmlFL As ClassADF = ClassADF.Carica(strFL)

            For Each xmlCon As ClassADF.ClassCONTROL In xmlFL.CONTROLS
                CreaControl(Me, scrMain, xmlCon)

            Next

            scrMain.AddCode(xmlFL.MODULO)

            My.Application.OpenForms.Item(xmlFL.PROPERTY.STARTCONTROL).ShowDialog(Me)

        End If

        Me.Close()

    End Sub

    Private Sub CreaControl(ByRef cltPadre As Control, ByRef scrPadre As MSScriptControl.ScriptControlClass, xmlControl As ClassADF.ClassCONTROL)

        If xmlControl IsNot Nothing Then

            Dim cltME As Control = Nothing

            Select Case xmlControl.PROPERTY.TYPE
                Case ClassADF.ClassCONTROL.ClassControlPROPERTY.Types.FORM

                    cltME = New Form With {
                        .Name = xmlControl.PROPERTY.NAME,
                        .Text = xmlControl.PROPERTY.TEXT,
                        .Width = xmlControl.PROPERTY.WIDTH,
                        .Height = xmlControl.PROPERTY.HEIGHT
                    }

                    scrPadre = New MSScriptControl.ScriptControlClass With {
                        .Language = "VBScript",
                        .AllowUI = True
                    }

                    cltME.Show()
                    cltME.Hide()

                Case ClassADF.ClassCONTROL.ClassControlPROPERTY.Types.BUTTON

                    cltME = New Button With {
                        .Name = xmlControl.PROPERTY.NAME,
                        .Text = xmlControl.PROPERTY.TEXT,
                        .Width = xmlControl.PROPERTY.WIDTH,
                        .Height = xmlControl.PROPERTY.HEIGHT,
                        .Top = xmlControl.PROPERTY.TOP,
                        .Left = xmlControl.PROPERTY.LEFT,
                        .Parent = cltPadre
                    }

            End Select

            If cltME IsNot Nothing Then

                scrPadre.AddObject(cltME.Name, cltME, True)

                If Not String.IsNullOrEmpty(xmlControl.EVENTS.CLICK) Then

                    Try
                        scrPadre.AddCode($"Sub {cltME.Name}_Click(){vbCrLf}{xmlControl.EVENTS.CLICK}{vbCrLf}End Sub{vbCrLf}")
                    Catch ex As Exception
                        MsgBox(ex.Message, MsgBoxStyle.Critical, "ADF")
                    End Try

                    AddHandler cltME.Click, AddressOf clt_Click

                End If

                For Each xmlCltCon As ClassADF.ClassCONTROL In xmlControl.CONTROLS
                    CreaControl(cltME, scrPadre, xmlCltCon)

                Next

                cltME.Tag = scrPadre

            End If
        End If

    End Sub

    Private Sub clt_Click(sender As System.Object, e As System.EventArgs)

        Try
            CType(CType(sender, Control).Tag, MSScriptControl.ScriptControl).Run(CType(sender, Control).Name & "_Click")
        Catch ex As Exception
            MsgBox(ex.Message, MsgBoxStyle.Critical, "ADF")
        End Try

    End Sub

End Class

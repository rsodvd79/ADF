<Serializable>
<System.Xml.Serialization.XmlRoot("ADF")>
Public Class ClassADF

    Public Class ClassMainPROPERTY
        Public Property STARTCONTROL As String = String.Empty
    End Class

    Public Class ClassCONTROL
        Public Class ClassControlPROPERTY
            Public Enum Types
                FORM
                BUTTON
            End Enum

            Public [TYPE] As Types = Types.FORM
            Public [NAME] As String = String.Empty
            Public [TEXT] As String = String.Empty
            Public [WIDTH] As Integer = 0
            Public [HEIGHT] As Integer = 0
            Public [TOP] As Integer = 0
            Public [LEFT] As Integer = 0
        End Class

        Public Class ClassEVENT
            Public [CLICK] As String = String.Empty
        End Class

        Public [PROPERTY] As ClassControlPROPERTY
        Public [EVENTS] As ClassEVENT
        <System.Xml.Serialization.XmlArrayItem(ElementName:="CONTROL")>
        Public [CONTROLS] As List(Of ClassCONTROL)

        Public Sub New()
            [PROPERTY] = New ClassControlPROPERTY
            [CONTROLS] = New List(Of ClassCONTROL)
            [EVENTS] = New ClassEVENT
        End Sub
    End Class

    Public [PROPERTY] As ClassMainPROPERTY
    Public [MODULO] As String
    <System.Xml.Serialization.XmlArrayItem(ElementName:="CONTROL")>
    Public [CONTROLS] As List(Of ClassCONTROL)

    Public Sub New()
        [PROPERTY] = New ClassMainPROPERTY
        [MODULO] = String.Empty
        [CONTROLS] = New List(Of ClassCONTROL)
    End Sub

    Public Function Clone() As ClassADF
        Return DirectCast(MemberwiseClone(), ClassADF)
    End Function

    Public Function Salva(FileXML As String) As Boolean

        Try
            Dim file As String = FileXML

            If IO.File.Exists(file) Then
                IO.File.Delete(file)
            End If

            Dim XmlWrt As Xml.XmlWriter = Xml.XmlWriter.Create(file, New Xml.XmlWriterSettings() With {.Indent = True})
            Dim Srlzr As New Xml.Serialization.XmlSerializer(GetType(ClassADF))
            Srlzr.Serialize(XmlWrt, Me)
            XmlWrt.Flush()
            XmlWrt.Close()

            Return True

        Catch ex As Exception
            Throw

        End Try

        Return False
    End Function

    Public Shared Function Carica(FileXML As String) As ClassADF
        Dim rtn As ClassADF = Nothing
        Dim file As String = FileXML

        Try
            If IO.File.Exists(file) Then
                Dim XmlRdr As Xml.XmlReader = Xml.XmlReader.Create(New IO.StringReader(IO.File.ReadAllText(file)))

                Dim Srlzr As New Xml.Serialization.XmlSerializer(GetType(ClassADF))
                ' AddHandler Srlzr.UnknownNode, AddressOf Srlzr_UnknownNode
                ' AddHandler Srlzr.UnknownAttribute, AddressOf Srlzr_UnknownAttribute

                If Srlzr.CanDeserialize(XmlRdr) Then
                    rtn = DirectCast(Srlzr.Deserialize(XmlRdr), ClassADF)

                End If

                XmlRdr.Close()

            Else
                rtn = New ClassADF()
                rtn.Salva(FileXML)

            End If

        Catch ex As Exception
            Throw

        End Try

        Return rtn
    End Function
End Class

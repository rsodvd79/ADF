using System;
using System.Collections.Generic;
using System.IO;
using System.Xml;
using System.Xml.Serialization;

namespace ADF;

[Serializable]
[XmlRoot("ADF")]
public class ClassADF
{
    public class ClassMainPROPERTY
    {
        public string STARTCONTROL { get; set; } = string.Empty;
    }

    public class ClassCONTROL
    {
        public class ClassControlPROPERTY
        {
            public enum Types
            {
                FORM,
                BUTTON
            }

            public Types TYPE = Types.FORM;
            public string NAME = string.Empty;
            public string TEXT = string.Empty;
            public int WIDTH = 0;
            public int HEIGHT = 0;
            public int TOP = 0;
            public int LEFT = 0;
        }

        public class ClassEVENT
        {
            public string CLICK = string.Empty;
        }

        public ClassControlPROPERTY PROPERTY;
        public ClassEVENT EVENTS;
        [XmlArrayItem(ElementName = "CONTROL")]
        public List<ClassCONTROL> CONTROLS;

        public ClassCONTROL()
        {
            PROPERTY = new ClassControlPROPERTY();
            CONTROLS = new List<ClassCONTROL>();
            EVENTS = new ClassEVENT();
        }
    }

    public ClassMainPROPERTY PROPERTY;
    public string MODULO;
    [XmlArrayItem(ElementName = "CONTROL")]
    public List<ClassCONTROL> CONTROLS;

    public ClassADF()
    {
        PROPERTY = new ClassMainPROPERTY();
        MODULO = string.Empty;
        CONTROLS = new List<ClassCONTROL>();
    }

    public ClassADF Clone() => (ClassADF)MemberwiseClone();

    public bool Salva(string fileXML)
    {
        try
        {
            if (File.Exists(fileXML))
                File.Delete(fileXML);

            using var xmlWrt = XmlWriter.Create(fileXML, new XmlWriterSettings { Indent = true });
            var srlzr = new XmlSerializer(typeof(ClassADF));
            srlzr.Serialize(xmlWrt, this);
            return true;
        }
        catch
        {
            throw;
        }
    }

    public static ClassADF Carica(string fileXML)
    {
        ClassADF? rtn = null;
        try
        {
            if (File.Exists(fileXML))
            {
                using var xmlRdr = XmlReader.Create(new StringReader(File.ReadAllText(fileXML)));
                var srlzr = new XmlSerializer(typeof(ClassADF));
                if (srlzr.CanDeserialize(xmlRdr))
                    rtn = (ClassADF)srlzr.Deserialize(xmlRdr)!;
            }
            else
            {
                rtn = new ClassADF();
                rtn.Salva(fileXML);
            }
        }
        catch
        {
            throw;
        }
        return rtn!;
    }
}

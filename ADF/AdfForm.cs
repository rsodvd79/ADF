using System;
using System.Linq;
using System.Windows.Forms;
using MSScriptControl;

namespace ADF;

public partial class AdfForm : Form
{
    private ScriptControlClass scrMain = new ScriptControlClass();

    public AdfForm()
    {
        InitializeComponent();

        AppDomain.CurrentDomain.UnhandledException += (sender, args) =>
        {
            try
            {
                MessageBox.Show(((Exception)args.ExceptionObject).Message, "ADF", MessageBoxButtons.OK, MessageBoxIcon.Error);
            }
            catch
            {
                // ignore
            }
        };
    }

    private void BtnOk_Click(object? sender, EventArgs e)
    {
        Close();
    }

    private void AdfForm_Load(object? sender, EventArgs e)
    {
        string strFL = string.Join(" ", Environment.GetCommandLineArgs().Skip(1)).Trim();

        scrMain.Language = "VBScript";
        scrMain.AllowUI = true;

        if (!string.IsNullOrEmpty(strFL) && System.IO.File.Exists(strFL))
        {
            var xmlFL = ClassADF.Carica(strFL);

            foreach (var xmlCon in xmlFL.CONTROLS)
                CreaControl(this, ref scrMain, xmlCon);

            scrMain.AddCode(xmlFL.MODULO);

            Application.OpenForms.Cast<Form>()
                .FirstOrDefault(f => f.Name == xmlFL.PROPERTY.STARTCONTROL)
                ?.ShowDialog(this);
        }

        Close();
    }

    private void CreaControl(Control cltPadre, ref ScriptControlClass scrPadre, ClassADF.ClassCONTROL xmlControl)
    {
        if (xmlControl == null) return;

        Control? cltME = null;

        switch (xmlControl.PROPERTY.TYPE)
        {
            case ClassADF.ClassCONTROL.ClassControlPROPERTY.Types.FORM:
                scrPadre = new ScriptControlClass
                {
                    Language = "VBScript",
                    AllowUI = true
                };
                cltME = new Form
                {
                    Name = xmlControl.PROPERTY.NAME,
                    Text = xmlControl.PROPERTY.TEXT,
                    Width = xmlControl.PROPERTY.WIDTH,
                    Height = xmlControl.PROPERTY.HEIGHT
                };
                cltME.Show();
                cltME.Hide();
                break;

            case ClassADF.ClassCONTROL.ClassControlPROPERTY.Types.BUTTON:
                cltME = new Button
                {
                    Name = xmlControl.PROPERTY.NAME,
                    Text = xmlControl.PROPERTY.TEXT,
                    Width = xmlControl.PROPERTY.WIDTH,
                    Height = xmlControl.PROPERTY.HEIGHT,
                    Top = xmlControl.PROPERTY.TOP,
                    Left = xmlControl.PROPERTY.LEFT,
                    Parent = cltPadre
                };
                break;
        }

        if (cltME != null)
        {
            scrPadre.AddObject(cltME.Name, cltME, true);

            if (!string.IsNullOrEmpty(xmlControl.EVENTS.CLICK))
            {
                try
                {
                    scrPadre.AddCode($"Sub {cltME.Name}_Click()\r\n{xmlControl.EVENTS.CLICK}\r\nEnd Sub\r\n");
                }
                catch (Exception ex)
                {
                    MessageBox.Show(ex.Message, "ADF", MessageBoxButtons.OK, MessageBoxIcon.Error);
                }

                cltME.Click += Clt_Click;
            }

            foreach (var xmlCltCon in xmlControl.CONTROLS)
                CreaControl(cltME, ref scrPadre, xmlCltCon);

            cltME.Tag = scrPadre;
        }
    }

    private void Clt_Click(object? sender, EventArgs e)
    {
        try
        {
            var ctrl = (Control)sender!;
            ((ScriptControlClass)ctrl.Tag!).Run(ctrl.Name + "_Click");
        }
        catch (Exception ex)
        {
            MessageBox.Show(ex.Message, "ADF", MessageBoxButtons.OK, MessageBoxIcon.Error);
        }
    }
}

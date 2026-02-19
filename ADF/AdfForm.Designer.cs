#nullable enable
namespace ADF;

partial class AdfForm
{
    private System.ComponentModel.IContainer? components = null;

    protected override void Dispose(bool disposing)
    {
        if (disposing && components != null)
            components.Dispose();
        base.Dispose(disposing);
    }

    private void InitializeComponent()
    {
        btnOk = new System.Windows.Forms.Button();
        lblTitolo = new System.Windows.Forms.Label();
        SuspendLayout();
        //
        // btnOk
        //
        btnOk.Location = new System.Drawing.Point(12, 45);
        btnOk.Name = "btnOk";
        btnOk.Size = new System.Drawing.Size(253, 41);
        btnOk.TabIndex = 0;
        btnOk.Text = "OK";
        btnOk.UseVisualStyleBackColor = true;
        btnOk.Click += BtnOk_Click;
        //
        // lblTitolo
        //
        lblTitolo.AutoSize = true;
        lblTitolo.Font = new System.Drawing.Font("Tahoma", 20.25f, System.Drawing.FontStyle.Bold | System.Drawing.FontStyle.Italic, System.Drawing.GraphicsUnit.Point, 0);
        lblTitolo.Location = new System.Drawing.Point(12, 9);
        lblTitolo.Name = "lblTitolo";
        lblTitolo.Size = new System.Drawing.Size(256, 33);
        lblTitolo.TabIndex = 1;
        lblTitolo.Text = "A.D.F ver. 0.1 Win";
        //
        // AdfForm
        //
        AutoScaleDimensions = new System.Drawing.SizeF(6f, 13f);
        AutoScaleMode = System.Windows.Forms.AutoScaleMode.Font;
        ClientSize = new System.Drawing.Size(279, 99);
        Controls.Add(lblTitolo);
        Controls.Add(btnOk);
        FormBorderStyle = System.Windows.Forms.FormBorderStyle.FixedDialog;
        MaximizeBox = false;
        MinimizeBox = false;
        Name = "AdfForm";
        StartPosition = System.Windows.Forms.FormStartPosition.CenterScreen;
        Text = "ADF";
        Load += AdfForm_Load;
        ResumeLayout(false);
        PerformLayout();
    }

    private System.Windows.Forms.Button btnOk = null!;
    private System.Windows.Forms.Label lblTitolo = null!;
}

import { exec } from "child_process";
import { IS_WIN } from "../utils/constants";

/**
 * Prompts the user for a pairing code using a system-native dialog.
 * @returns A promise that resolves to the pairing code or "CANCELLED".
 */
export const getPairingCode = async (): Promise<string> => {
  const title = "T_Music_Bot RPC Setup";
  const msg =
    "'1. Run [/rpc connect] in a Discord channel.' + [char]13 + [char]10 + '2. Paste the code given below.'";

  if (IS_WIN) {
    const ps = `powershell -NoProfile -Command "Add-Type -AssemblyName System.Windows.Forms; Add-Type -AssemblyName System.Drawing; [Windows.Forms.Application]::EnableVisualStyles(); $f=New-Object Windows.Forms.Form; $f.Text='${title}'; $f.Size=New-Object Drawing.Size(420,300); $f.StartPosition='CenterScreen'; $f.FormBorderStyle='FixedDialog'; $f.Topmost=$true; $f.Font=New-Object Drawing.Font('Segoe UI', 10); $l1=New-Object Windows.Forms.Label; $l1.Text='Instructions:'; $l1.Font=New-Object Drawing.Font('Segoe UI', 10, [Drawing.FontStyle]::Bold); $l1.Location=New-Object Drawing.Point(20,20); $l2=New-Object Windows.Forms.Label; $l2.Text=(${msg}); $l2.Size=New-Object Drawing.Size(380,50); $l2.Location=New-Object Drawing.Point(20,45); $l3=New-Object Windows.Forms.Label; $l3.Text='Enter Code:'; $l3.Font=New-Object Drawing.Font('Segoe UI', 10, [Drawing.FontStyle]::Bold); $l3.Location=New-Object Drawing.Point(20,105); $t=New-Object Windows.Forms.TextBox; $t.Location=New-Object Drawing.Point(22,130); $t.Size=New-Object Drawing.Size(360,25); $btnOk=New-Object Windows.Forms.Button; $btnOk.Text='Connect'; $btnOk.Location=New-Object Drawing.Point(195,190); $btnOk.DialogResult=1; $btnCan=New-Object Windows.Forms.Button; $btnCan.Text='Cancel'; $btnCan.Location=New-Object Drawing.Point(300,190); $btnCan.DialogResult=2; $f.Controls.AddRange(@($l1,$l2,$l3,$t,$btnOk,$btnCan)); $f.Activate(); if($f.ShowDialog()-eq1){$t.Text}else{'CANCELLED'}"`;
    return await new Promise((r) =>
      exec(ps, { windowsHide: true }, (e, o) =>
        r(o ? o.trim() : "CANCELLED"),
      ),
    );
  } else {
    return await new Promise((r) =>
      exec(
        `zenity --entry --title="${title}" --text="Paste Pairing Code:"`,
        (e, o) => r(o ? o.trim() : "CANCELLED"),
      ),
    );
  }
};

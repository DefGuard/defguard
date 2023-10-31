{% extends "base.tera" %}
{% block mail_content %}
  <div style="background-color:#f9f9f9;">
    <!--[if mso | IE]><table align="center" border="0" cellpadding="0" cellspacing="0" class="" role="presentation" style="width:600px;" width="600" bgcolor="#f9f9f9" ><tr><td style="line-height:0px;font-size:0px;mso-line-height-rule:exactly;"><![endif]-->
    <!--[if mso | IE]></td></tr></table><table align="center" border="0" cellpadding="0" cellspacing="0" class="" role="presentation" style="width:600px;" width="600" bgcolor="#f9f9f9" ><tr><td style="line-height:0px;font-size:0px;mso-line-height-rule:exactly;"><![endif]-->
    <div style="background:#f9f9f9;background-color:#f9f9f9;margin:0px auto;max-width:600px;">
      <table align="center" border="0" cellpadding="0" cellspacing="0" role="presentation" style="background:#f9f9f9;background-color:#f9f9f9;width:100%;">
        <tbody>
          <tr>
            <td style="direction:ltr;font-size:0px;padding:20px 0;text-align:center;">
              <!--[if mso | IE]><table role="presentation" border="0" cellpadding="0" cellspacing="0"><tr><td class="" style="vertical-align:top;width:600px;" ><![endif]-->
              <div class="mj-column-per-100 mj-outlook-group-fix" style="font-size:0px;text-align:left;direction:ltr;display:inline-block;vertical-align:top;width:100%;">
                <table border="0" cellpadding="0" cellspacing="0" role="presentation" style="vertical-align:top;" width="100%">
                  <tbody>
                    <tr>
                      <td align="center" style="font-size:0px;padding:10px 25px;word-break:break-word;">
                        <div style="font-family:Roboto;font-size:12px;line-height:1;text-align:center;color:#000000;">
                          <p>You're receiving this email to configure new desktop client.</p>
                        </div>
                      </td>
                    </tr>
                    <tr>
                      <td align="center" style="font-size:0px;padding:10px 25px;word-break:break-word;">
                        <div style="font-family:Roboto;font-size:12px;line-height:1;text-align:center;color:#899CA8;">
                          <p style="font-family:Roboto;font-size:12px;line-height:1;text-align:center;color:#000000;">Please paste url and token in your desktop app:</p>
                          <p style="font-family:Roboto;font-size:12px;line-height:1;text-align:center;color:#000000;">Url:</p>
                          <a style="color:#899CA8;" target="_blank">{{ url }}</a>
                          <p style="font-family:Roboto;font-size:12px;line-height:1;text-align:center;color:#000000;">Token:</p>
                          <a style="color:#899CA8;" target="_blank">{{ token }}</a>
                        </div>
                      </td>
                    </tr>
                  </tbody>
                </table>
              </div>
              <!--[if mso | IE]></td></tr></table><![endif]-->
            </td>
          </tr>
        </tbody>
      </table>
    </div>
    <!--[if mso | IE]></td></tr></table><![endif]-->
  </div>
{% endblock content %}

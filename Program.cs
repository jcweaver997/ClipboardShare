using System;
using System.Collections.Generic;
using System.Linq;
using System.Net;
using System.Net.Sockets;
using System.Text;
using System.Threading;
using System.Windows;
using TextCopy;

namespace ClipboardShare
{
    class Program
    {
        static UdpClient udpClient;
        static IPEndPoint groupEp = new IPEndPoint(IPAddress.Parse("234.21.76.126"),8473);
        static byte[] secretCode = { 44,76,123,65};
        

        static void Main(string[] args)
        {
            udpClient = new UdpClient(groupEp.Port);
            udpClient.JoinMulticastGroup(groupEp.Address);
            
            
            new Thread(() => { Listen(); }).Start();
            CheckClipBoardChange();
        }

        static void CheckClipBoardChange()
        {
            string cb = ClipboardService.GetText();
            while (true)
            {
                Thread.Sleep(100);
                string cbt = ClipboardService.GetText();
                if (cbt == null) { continue; }
                if (cb != cbt && cbt.Length > 0)
                {
                    Console.WriteLine($"Clipboard changed: {cbt}");
                    cb = cbt;
                    byte[] cbb = Encoding.UTF8.GetBytes(cb);
                    byte[] message = new byte[4+cbb.Length];
                    secretCode.CopyTo(message,0);
                    cbb.CopyTo(message, 4);
                    udpClient.Send(message, message.Length, groupEp);
                }
            }
        }

        static void Listen()
        {
            while (true)
            {
                IPEndPoint ep = new IPEndPoint(IPAddress.Any, 0);
                byte[] m = udpClient.Receive(ref ep);
                Console.WriteLine($"Got new bytes: {Encoding.UTF8.GetString(m)}");
                if (m.Length >= secretCode.Length+1)
                {
                    Console.WriteLine($"longer");
                    if (m[0..4].SequenceEqual(secretCode))
                    {
                        string newcb = Encoding.UTF8.GetString(m[4..]);
                        Console.WriteLine($"Got new text: {newcb}");
                        ClipboardService.SetText(newcb);
                    }
                }
                else
                {
                    Console.WriteLine("broadcast too short " + m.Length);
                }
            }

        }
    }
}

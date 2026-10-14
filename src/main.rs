use serde::Serialize;
use std::io::Write;
use std::process::{Command, Stdio};
use std::thread;

#[derive(Serialize)]
struct WaybarOutput {
    text: String,
    tooltip: String,
    class: String,
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let modo_waybar = args.iter().any(|arg| arg == "--waybar");
    let modo_rofi = args.iter().any(|arg| arg == "--rofi");

    let pacman_thread = thread::spawn(|| {
        Command::new("checkupdates").output()
    });

    let aur_thread = thread::spawn(|| {
        Command::new("paru").arg("-Qua").output()
    });

    let mut paquetes = Vec::new();

    if let Ok(Ok(pacman_out)) = pacman_thread.join() {
        if let Ok(texto) = String::from_utf8(pacman_out.stdout) {
            paquetes.extend(procesar_salida(&texto));
        }
    }

    if let Ok(Ok(aur_out)) = aur_thread.join() {
        if let Ok(texto) = String::from_utf8(aur_out.stdout) {
            paquetes.extend(procesar_salida(&texto));
        }
    }

    let total = paquetes.len();

    if total == 0 {
        if modo_waybar {
            imprimir_waybar(0, "Sistema actualizado", "updated");
        } else if modo_rofi {
            Command::new("notify-send")
                .args(["-u", "low", "Actualizaciones", "El sistema ya está al día."])
                .spawn()
                .unwrap();
        } else {
            println!("✅ Sistema completamente actualizado (0 paquetes).");
        }
        return;
    }

    let mut criticos = Vec::new();
    let mut entorno = Vec::new();
    let mut normales = Vec::new();

    for pkg in paquetes {
        let nombre = pkg.split_whitespace().next().unwrap_or("").to_lowercase();
        
        if nombre.contains("linux") || nombre.contains("nvidia") || nombre.contains("mesa") || nombre.contains("systemd") || nombre.contains("glibc") {
            criticos.push(format!("🔴 [NÚCLEO/DRIVERS] {}", pkg));
        } else if nombre.contains("hypr") || nombre.contains("wayland") || nombre.contains("waybar") || nombre.contains("xdg") {
            entorno.push(format!("🟡 [ENTORNO] {}", pkg));
        } else {
            normales.push(format!("📦 {}", pkg));
        }
    }

    if modo_waybar {
        let mut tooltip = format!("Actualizaciones pendientes: {}\n", total);
        tooltip.push_str("---------------------------\n");
        
        for c in &criticos { tooltip.push_str(&format!("{}\n", c)); }
        for e in &entorno { tooltip.push_str(&format!("{}\n", e)); }
        for n in normales.iter().take(15) { tooltip.push_str(&format!("{}\n", n)); }
        
        if normales.len() > 15 {
            tooltip.push_str(&format!("... y {} más", normales.len() - 15));
        }

        let clase = if !criticos.is_empty() {
            "critical"
        } else if !entorno.is_empty() {
            "warning"
        } else {
            "normal"
        };

        imprimir_waybar(total, &tooltip.trim(), clase);
    }
    else if modo_rofi {
        let mut opciones = String::new();
        for c in &criticos { opciones.push_str(&format!("{}\n", c)); }
        for e in &entorno { opciones.push_str(&format!("{}\n", e)); }
        for n in &normales { opciones.push_str(&format!("{}\n", n)); }

        let mut child = Command::new("rofi")
            .args(["-dmenu", "-i", "-p", "🔄 Actualizar", "-no-custom"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .expect("Fallo al ejecutar rofi");

        if let Some(stdin) = child.stdin.as_mut() {
            stdin.write_all(opciones.as_bytes()).expect("Fallo al escribir en rofi");
        }

        let output = child.wait_with_output().expect("Fallo al leer rofi");
        let seleccion = String::from_utf8_lossy(&output.stdout).trim().to_string();

        if !seleccion.is_empty() {
            let partes: Vec<&str> = seleccion.split_whitespace().collect();
            let mut nombre_paquete = "";
            
            for p in partes {
                if !p.starts_with('[') && p != "🔴" && p != "🟡" && p != "📦" {
                    nombre_paquete = p;
                    break;
                }
            }

            if !nombre_paquete.is_empty() {
                Command::new("kitty")
                    .args(["--class", "update-term", "-e", "paru", "-S", nombre_paquete])
                    .spawn()
                    .expect("Fallo al lanzar kitty");
            }
        }
    } else {
        for c in criticos { println!("{}", c); }
        for e in entorno { println!("{}", e); }
        for n in normales { println!("{}", n); }
    }
}

fn procesar_salida(texto: &str) -> Vec<String> {
    texto.lines()
        .map(|line| line.trim().to_string())
        .filter(|line| !line.is_empty())
        .collect()
}

fn imprimir_waybar(cantidad: usize, tooltip: &str, clase: &str) {
    let icon = match clase {
        "critical" => "🚨",
        "warning" => "⚠️",
        "updated" => "✔️",
        _ => "📦",
    };

    let text = if cantidad > 0 { format!("{} {}", icon, cantidad) } else { "".to_string() };

    let output = WaybarOutput {
        text,
        tooltip: tooltip.to_string(),
        class: clase.to_string(),
    };

    println!("{}", serde_json::to_string(&output).unwrap());
}

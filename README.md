# Rust Synth

Un sintetizador de audio escrito en Rust con soporte para MIDI y una interfaz gráfica.

## Características

- Sintetizador polifónico con oscilador de tabla de ondas
- Soporte para entrada MIDI
- Filtro paso bajo
- Envolvente ADSR (Attack, Decay, Sustain, Release)
- Interfaz gráfica para configuración
- Modo consola para uso tradicional
- Optimizado para bajo uso de CPU

## Requisitos

- Rust y Cargo (versión 1.70.0 o superior)
- Controlador MIDI (opcional)
- Dispositivo de audio compatible

## Instalación

1. Clona este repositorio:
   ```
   git clone https://github.com/salogelTorres/rust-synth.git
   cd rust-synth
   ```

2. Compila el proyecto:
   ```
   cargo build --release
   ```

## Uso

### Interfaz Gráfica

Para iniciar el sintetizador con la interfaz gráfica:

```
cargo run --release -- --gui
```

La interfaz gráfica permite:
- Seleccionar el host de audio (ASIO, WASAPI, etc.)
- Seleccionar el dispositivo de salida de audio
- Seleccionar la frecuencia de muestreo
- Ajustar el volumen
- Conectar/desconectar dispositivos MIDI
- Iniciar/detener el sintetizador

### Modo Consola

Para iniciar el sintetizador en modo consola:

```
cargo run --release
```

En el modo consola:
1. Selecciona el host de audio (ASIO recomendado para menor latencia)
2. Selecciona el dispositivo de salida de audio
3. El sintetizador se iniciará automáticamente
4. Usa tu controlador MIDI para tocar notas
5. Presiona Ctrl+C para salir

## Optimizaciones

El sintetizador está optimizado para un rendimiento eficiente:
- Procesamiento de audio por bloques para reducir operaciones de bloqueo
- Interpolación lineal para la tabla de ondas
- Precálculo de coeficientes para filtros
- Optimización de la envolvente ADSR

## Solución de problemas

### ASIO

Si estás usando ASIO:
- Asegúrate de tener instalado ASIO4ALL u otro driver ASIO
- Abre el panel de control de ASIO4ALL antes de iniciar el sintetizador
- Configura correctamente tu dispositivo de audio en el panel de control

### Problemas de audio

- Si experimentas cortes o latencia alta, intenta aumentar el tamaño del buffer
- Si no hay sonido, verifica que el dispositivo de salida esté correctamente seleccionado
- Asegúrate de que tu controlador MIDI esté conectado antes de iniciar el programa

## Licencia

Este proyecto está licenciado bajo la Licencia MIT - ver el archivo LICENSE para más detalles. 
# Plan: Panel de Filtrado

## Overview
Agregar un panel inferior para filtrar líneas del archivo de log, manteniendo la vista original.

## Decisiones de Diseño

### UI/UX
- **Ubicación**: Panel horizontal en la parte inferior
- **Divisor**: Inicia en 30% de la altura, redimensionable
- **Panel**: Siempre visible, vacío hasta aplicar primer filtro
- **Input**: Campo de texto - filtro se aplica al presionar ENTER

### Filtrado
- Texto simple enviado tal cual al core (sin conversión a regex por ahora)
- El backend maneja la lógica de filtrado

### Componentes
- Reutilizar `ContentLines` y `ContentScroll` tal cual existen
- Crear señales y recursos separados para el panel de filtro
- No hace falta nuevo contexto - se reutiliza `LogViewContext` con signals específicas del filtro

---

## Tareas

### Fase 1: API de Filtrado (Backend)

#### Tarea 1.1: Agregar structs de request
**Archivos**: `logmancer-web/src/api/commons.rs`

Agregar:
```rust
#[derive(Serialize,Deserialize,Debug)]
pub struct ApplyFilterRequest {
    pub file_id: String,
    pub filter: String
}

#[derive(Serialize,Deserialize,Debug)]
pub struct ReadFilterRequest {
    pub file_id: String,
    pub start_line: usize,
    pub max_lines: usize
}
```

#### Tarea 1.2: Crear handlers de filtro
**Archivos**: `logmancer-web/src/api/filter.rs` (nuevo)

Crear handlers:
- `apply_filter` - POST `/apply-filter` - envía el filtro al core
- `read_filter_page` - GET `/read-filter-page` - devuelve página filtrada

#### Tarea 1.3: Registrar rutas
**Archivos**: `logmancer-web/src/api/config.rs`, `logmancer-web/src/api/mod.rs`

Agregar rutas:
- POST `/apply-filter`
- GET `/read-filter-page`

---

### Fase 2: Frontend - API Client

#### Tarea 2.1: Agregar funciones fetch
**Archivos**: `logmancer-web/src/components/async_functions.rs`

Agregar:
- `apply_filter(file_id, filter)` - envía texto al endpoint
- `fetch_filter_page(file_id, start_line, max_lines)` - trae página filtrada

---

### Fase 3: Componente FilterPane

#### Tarea 3.1: Crear FilterPane
**Archivos**: `logmancer-web/src/components/filter_pane.rs` (nuevo)

Crear componente que:
- Tenga campo de texto para input del filtro
- Reutilice `ContentLines` y `ContentScroll` 
- Tenga signals locales propias: `filter_start_line`, `filter_page_size`
- Tenga `LocalResource` que llame a `fetch_filter_page`
- Genere un `LogViewContext` con esos datos y lo pase a los componentes hijos

#### Tarea 3.2: Agregar FilterPane al modulo
**Archivos**: `logmancer-web/src/components/mod.rs`

Exportar el nuevo componente

---

### Fase 4: Integración en LogView

#### Tarea 4.1: Agregar FilterPane a LogView
**Archivos**: `logmancer-web/src/components/log_view.rs`

- Importar `FilterPane`
- Agregar estructura con divisor: `MainPane` + divisor + `FilterPane`
- Estilos CSS para el divisor redimensionable

#### Tarea 4.2: Estilos del divisor
**Archivos**: `logmancer-web/src/...` (css del proyecto)

- Panel superior: ~70% de altura (flex)
- Divisor: cursor para redimensionar, fondo visual
- Panel inferior: ~30% de altura (flex)

---

## Pendientes (No para esta iteración)
- [ ] Colapsar el panel de filtro
- [ ] Persistir altura del divisor
- [ ] Convertir texto a regex (ej: `*` -> `.*`)
- [ ] Modo case-sensitive / case-insensitive

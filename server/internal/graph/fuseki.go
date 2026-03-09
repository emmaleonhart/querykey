package graph

import (
	"bytes"
	"context"
	"encoding/json"
	"fmt"
	"io"
	"log"
	"net/http"
	"net/url"
	"strings"
	"time"

	"github.com/google/uuid"
	"github.com/secretarybird/server/internal/models"
)

const (
	// RDF namespace for Secretarybird entities
	nsBase = "http://secretarybird.dev/ns/"
	nsPerson = nsBase + "person/"
	nsTask   = nsBase + "task/"
	nsEvent  = nsBase + "event/"
	nsMsg    = nsBase + "message/"
)

// FusekiClient manages the connection to Apache Jena Fuseki triple store.
type FusekiClient struct {
	baseURL   string
	dataset   string
	client    *http.Client
}

// NewFusekiClient creates a Fuseki client.
func NewFusekiClient(baseURL, dataset string) *FusekiClient {
	return &FusekiClient{
		baseURL: strings.TrimRight(baseURL, "/"),
		dataset: dataset,
		client: &http.Client{
			Timeout: 30 * time.Second,
		},
	}
}

// Ping checks if Fuseki is reachable and the dataset exists.
func (f *FusekiClient) Ping(ctx context.Context) error {
	req, err := http.NewRequestWithContext(ctx, "GET",
		fmt.Sprintf("%s/$/ping", f.baseURL), nil)
	if err != nil {
		return err
	}
	resp, err := f.client.Do(req)
	if err != nil {
		return fmt.Errorf("Fuseki not reachable: %w", err)
	}
	defer resp.Body.Close()
	io.Copy(io.Discard, resp.Body)

	if resp.StatusCode != 200 {
		return fmt.Errorf("Fuseki returned %d", resp.StatusCode)
	}
	return nil
}

// EnsureDataset creates the dataset if it doesn't exist.
func (f *FusekiClient) EnsureDataset(ctx context.Context) error {
	// Check if dataset exists
	req, err := http.NewRequestWithContext(ctx, "GET",
		fmt.Sprintf("%s/$/datasets/%s", f.baseURL, f.dataset), nil)
	if err != nil {
		return err
	}
	resp, err := f.client.Do(req)
	if err != nil {
		return err
	}
	defer resp.Body.Close()
	io.Copy(io.Discard, resp.Body)

	if resp.StatusCode == 200 {
		log.Printf("[fuseki] dataset '%s' exists", f.dataset)
		return nil
	}

	// Create dataset
	form := url.Values{}
	form.Set("dbName", f.dataset)
	form.Set("dbType", "tdb2")

	req, err = http.NewRequestWithContext(ctx, "POST",
		fmt.Sprintf("%s/$/datasets", f.baseURL),
		strings.NewReader(form.Encode()))
	if err != nil {
		return err
	}
	req.Header.Set("Content-Type", "application/x-www-form-urlencoded")

	resp, err = f.client.Do(req)
	if err != nil {
		return fmt.Errorf("failed to create dataset: %w", err)
	}
	defer resp.Body.Close()
	io.Copy(io.Discard, resp.Body)

	log.Printf("[fuseki] created dataset '%s'", f.dataset)
	return nil
}

// SPARQLResult represents a SPARQL query result.
type SPARQLResult struct {
	Head    SPARQLHead    `json:"head"`
	Results SPARQLBindings `json:"results"`
}

type SPARQLHead struct {
	Vars []string `json:"vars"`
}

type SPARQLBindings struct {
	Bindings []map[string]SPARQLValue `json:"bindings"`
}

type SPARQLValue struct {
	Type     string `json:"type"`
	Value    string `json:"value"`
	DataType string `json:"datatype,omitempty"`
}

// Query executes a SPARQL SELECT query.
func (f *FusekiClient) Query(ctx context.Context, sparql string) (*SPARQLResult, error) {
	req, err := http.NewRequestWithContext(ctx, "POST",
		fmt.Sprintf("%s/%s/sparql", f.baseURL, f.dataset),
		strings.NewReader("query="+url.QueryEscape(sparql)))
	if err != nil {
		return nil, err
	}
	req.Header.Set("Content-Type", "application/x-www-form-urlencoded")
	req.Header.Set("Accept", "application/sparql-results+json")

	resp, err := f.client.Do(req)
	if err != nil {
		return nil, fmt.Errorf("SPARQL query failed: %w", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != 200 {
		body, _ := io.ReadAll(resp.Body)
		return nil, fmt.Errorf("SPARQL returned %d: %s", resp.StatusCode, string(body))
	}

	var result SPARQLResult
	if err := json.NewDecoder(resp.Body).Decode(&result); err != nil {
		return nil, err
	}
	return &result, nil
}

// Update executes a SPARQL UPDATE (INSERT/DELETE).
func (f *FusekiClient) Update(ctx context.Context, sparql string) error {
	req, err := http.NewRequestWithContext(ctx, "POST",
		fmt.Sprintf("%s/%s/update", f.baseURL, f.dataset),
		strings.NewReader("update="+url.QueryEscape(sparql)))
	if err != nil {
		return err
	}
	req.Header.Set("Content-Type", "application/x-www-form-urlencoded")

	resp, err := f.client.Do(req)
	if err != nil {
		return fmt.Errorf("SPARQL update failed: %w", err)
	}
	defer resp.Body.Close()
	io.Copy(io.Discard, resp.Body)

	if resp.StatusCode >= 400 {
		body, _ := io.ReadAll(resp.Body)
		return fmt.Errorf("SPARQL update returned %d: %s", resp.StatusCode, string(body))
	}
	return nil
}

// InsertTriples inserts N-Triples data into the graph.
func (f *FusekiClient) InsertTriples(ctx context.Context, triples string) error {
	req, err := http.NewRequestWithContext(ctx, "POST",
		fmt.Sprintf("%s/%s/data", f.baseURL, f.dataset),
		bytes.NewBufferString(triples))
	if err != nil {
		return err
	}
	req.Header.Set("Content-Type", "application/n-triples")

	resp, err := f.client.Do(req)
	if err != nil {
		return err
	}
	defer resp.Body.Close()
	io.Copy(io.Discard, resp.Body)

	if resp.StatusCode >= 400 {
		body, _ := io.ReadAll(resp.Body)
		return fmt.Errorf("triple insert returned %d: %s", resp.StatusCode, string(body))
	}
	return nil
}

// --- Entity Operations ---

// StorePerson inserts or updates a Person in the graph.
func (f *FusekiClient) StorePerson(ctx context.Context, p *models.Person) error {
	uri := nsPerson + url.PathEscape(p.ID)
	sparql := fmt.Sprintf(`
PREFIX sb: <%s>
DELETE WHERE { <%s> ?p ?o };
INSERT DATA {
  <%s> a sb:Person ;
    sb:displayName "%s" ;
    sb:createdAt "%s"^^<http://www.w3.org/2001/XMLSchema#dateTime> .
}`, nsBase, uri, uri,
		escapeSPARQL(p.DisplayName),
		p.CreatedAt.Format(time.RFC3339))

	// Add handles
	for _, h := range p.Handles {
		sparql += fmt.Sprintf(`
INSERT DATA {
  <%s> sb:handle [
    sb:platform "%s" ;
    sb:identifier "%s"
  ] .
}`, uri, escapeSPARQL(h.Platform), escapeSPARQL(h.Identifier))
	}

	return f.Update(ctx, sparql)
}

// StoreTask inserts or updates a Task in the graph.
func (f *FusekiClient) StoreTask(ctx context.Context, t *models.Task) error {
	uri := nsTask + t.ID.String()
	sparql := fmt.Sprintf(`
PREFIX sb: <%s>
DELETE WHERE { <%s> ?p ?o };
INSERT DATA {
  <%s> a sb:Task ;
    sb:title "%s" ;
    sb:description "%s" ;
    sb:status "%s" ;
    sb:confidence %f ;
    sb:ambiguityScore %f ;
    sb:createdAt "%s"^^<http://www.w3.org/2001/XMLSchema#dateTime> ;
    sb:updatedAt "%s"^^<http://www.w3.org/2001/XMLSchema#dateTime> .
`, nsBase, uri, uri,
		escapeSPARQL(t.Title),
		escapeSPARQL(t.Description),
		string(t.Status),
		t.Confidence,
		t.AmbiguityScore,
		t.CreatedAt.Format(time.RFC3339),
		t.UpdatedAt.Format(time.RFC3339))

	if t.AssignedTo != "" {
		sparql += fmt.Sprintf("  <%s> sb:assignedTo <%s%s> .\n", uri, nsPerson, url.PathEscape(t.AssignedTo))
	}
	if t.AssignedBy != "" {
		sparql += fmt.Sprintf("  <%s> sb:assignedBy <%s%s> .\n", uri, nsPerson, url.PathEscape(t.AssignedBy))
	}
	if t.Deadline != nil {
		sparql += fmt.Sprintf("  <%s> sb:deadline \"%s\"^^<http://www.w3.org/2001/XMLSchema#dateTime> .\n",
			uri, t.Deadline.Format(time.RFC3339))
	}
	for _, msgID := range t.SourceMessages {
		sparql += fmt.Sprintf("  <%s> sb:sourceMessage <%s%s> .\n", uri, nsMsg, msgID)
	}

	sparql += "}"
	return f.Update(ctx, sparql)
}

// StoreMessage inserts a Message in the graph.
func (f *FusekiClient) StoreMessage(ctx context.Context, m *models.Message) error {
	uri := nsMsg + m.ID.String()
	sparql := fmt.Sprintf(`
PREFIX sb: <%s>
INSERT DATA {
  <%s> a sb:Message ;
    sb:content "%s" ;
    sb:author <%s%s> ;
    sb:sourceIngest "%s" ;
    sb:confidence %f .
}`, nsBase, uri,
		escapeSPARQL(m.Content),
		nsPerson, url.PathEscape(m.Author),
		escapeSPARQL(m.SourceIngest),
		m.Confidence)

	return f.Update(ctx, sparql)
}

// StoreConflict inserts a Conflict in the graph.
func (f *FusekiClient) StoreConflict(ctx context.Context, c *models.Conflict) error {
	uri := nsBase + "conflict/" + c.ID.String()
	sparql := fmt.Sprintf(`
PREFIX sb: <%s>
INSERT DATA {
  <%s> a sb:Conflict ;
    sb:conflictType "%s" ;
    sb:messageA <%s%s> ;
    sb:messageB <%s%s> ;
    sb:explanation "%s" ;
    sb:resolution "%s" ;
    sb:createdAt "%s"^^<http://www.w3.org/2001/XMLSchema#dateTime> .
}`, nsBase, uri,
		string(c.Type),
		nsMsg, c.MessageA,
		nsMsg, c.MessageB,
		escapeSPARQL(c.Explanation),
		string(c.Resolution),
		c.CreatedAt.Format(time.RFC3339))

	return f.Update(ctx, sparql)
}

// GetTasksForPerson returns all tasks assigned to a person.
func (f *FusekiClient) GetTasksForPerson(ctx context.Context, personID string) ([]models.Task, error) {
	sparql := fmt.Sprintf(`
PREFIX sb: <%s>
SELECT ?id ?title ?description ?status ?confidence ?deadline ?createdAt ?updatedAt
WHERE {
  ?task a sb:Task ;
    sb:assignedTo <%s%s> ;
    sb:title ?title ;
    sb:description ?description ;
    sb:status ?status ;
    sb:confidence ?confidence ;
    sb:createdAt ?createdAt ;
    sb:updatedAt ?updatedAt .
  BIND(REPLACE(STR(?task), "%s", "") AS ?id)
  OPTIONAL { ?task sb:deadline ?deadline }
}`, nsBase, nsPerson, url.PathEscape(personID), nsTask)

	result, err := f.Query(ctx, sparql)
	if err != nil {
		return nil, err
	}

	var tasks []models.Task
	for _, binding := range result.Results.Bindings {
		t := models.Task{
			Title:       binding["title"].Value,
			Description: binding["description"].Value,
			Status:      models.TaskStatus(binding["status"].Value),
			Confidence:  parseFloat(binding["confidence"].Value),
			AssignedTo:  personID,
		}
		if id, err := uuid.Parse(binding["id"].Value); err == nil {
			t.ID = id
		}
		if created, err := time.Parse(time.RFC3339, binding["createdAt"].Value); err == nil {
			t.CreatedAt = created
		}
		if updated, err := time.Parse(time.RFC3339, binding["updatedAt"].Value); err == nil {
			t.UpdatedAt = updated
		}
		if dl, ok := binding["deadline"]; ok {
			if deadline, err := time.Parse(time.RFC3339, dl.Value); err == nil {
				t.Deadline = &deadline
			}
		}
		tasks = append(tasks, t)
	}
	return tasks, nil
}

// GetUnresolvedConflicts returns all conflicts with status "unresolved".
func (f *FusekiClient) GetUnresolvedConflicts(ctx context.Context) ([]models.Conflict, error) {
	sparql := fmt.Sprintf(`
PREFIX sb: <%s>
SELECT ?id ?type ?messageA ?messageB ?explanation ?createdAt
WHERE {
  ?conflict a sb:Conflict ;
    sb:conflictType ?type ;
    sb:messageA ?messageA ;
    sb:messageB ?messageB ;
    sb:explanation ?explanation ;
    sb:resolution "unresolved" ;
    sb:createdAt ?createdAt .
  BIND(REPLACE(STR(?conflict), "%sconflict/", "") AS ?id)
}`, nsBase, nsBase)

	result, err := f.Query(ctx, sparql)
	if err != nil {
		return nil, err
	}

	var conflicts []models.Conflict
	for _, binding := range result.Results.Bindings {
		c := models.Conflict{
			Type:        models.ConflictType(binding["type"].Value),
			MessageA:    binding["messageA"].Value,
			MessageB:    binding["messageB"].Value,
			Explanation: binding["explanation"].Value,
			Resolution:  models.ResolutionUnresolved,
		}
		if id, err := uuid.Parse(binding["id"].Value); err == nil {
			c.ID = id
		}
		if created, err := time.Parse(time.RFC3339, binding["createdAt"].Value); err == nil {
			c.CreatedAt = created
		}
		conflicts = append(conflicts, c)
	}
	return conflicts, nil
}

// GetAllPersons returns all persons in the graph.
func (f *FusekiClient) GetAllPersons(ctx context.Context) ([]models.Person, error) {
	sparql := fmt.Sprintf(`
PREFIX sb: <%s>
SELECT ?id ?displayName ?createdAt
WHERE {
  ?person a sb:Person ;
    sb:displayName ?displayName ;
    sb:createdAt ?createdAt .
  BIND(REPLACE(STR(?person), "%s", "") AS ?id)
}`, nsBase, nsPerson)

	result, err := f.Query(ctx, sparql)
	if err != nil {
		return nil, err
	}

	var persons []models.Person
	for _, binding := range result.Results.Bindings {
		p := models.Person{
			ID:          binding["id"].Value,
			DisplayName: binding["displayName"].Value,
		}
		if created, err := time.Parse(time.RFC3339, binding["createdAt"].Value); err == nil {
			p.CreatedAt = created
		}
		persons = append(persons, p)
	}
	return persons, nil
}

func escapeSPARQL(s string) string {
	s = strings.ReplaceAll(s, "\\", "\\\\")
	s = strings.ReplaceAll(s, "\"", "\\\"")
	s = strings.ReplaceAll(s, "\n", "\\n")
	s = strings.ReplaceAll(s, "\r", "\\r")
	return s
}

func parseFloat(s string) float64 {
	var f float64
	fmt.Sscanf(s, "%f", &f)
	return f
}
